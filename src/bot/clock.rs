use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use tokio::sync::Notify;

use super::{Bot, meet_lifecycle::end_meets_that_are_over, meet_request::drop_meet_requests_that_are_over};
use crate::time::now_unix_seconds;

const CHECK_EVERY: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
pub struct MeetClock {
    started: AtomicBool,
    restart: Notify,
}

impl MeetClock {
    pub fn check_now(bot: &Arc<Bot>) {
        if bot.meet_clock.started.swap(true, Ordering::Relaxed) {
            bot.meet_clock.restart.notify_one();
            return;
        }
        let bot = Arc::clone(bot);
        tokio::spawn(async move {
            let clock = &bot.meet_clock;
            repeat_checks(CHECK_EVERY, &clock.restart, || check_and_log(&bot)).await;
        });
    }
}

async fn check(bot: &Bot) -> anyhow::Result<()> {
    let now = now_unix_seconds();
    end_meets_that_are_over(bot, now).await?;
    drop_meet_requests_that_are_over(bot, now).await
}

async fn check_and_log(bot: &Bot) {
    if let Err(error) = check(bot).await {
        tracing::error!(?error, "Meet check failed");
    }
}

async fn repeat_checks<F, Checked>(period: Duration, restart: &Notify, mut check: F)
where
    F: FnMut() -> Checked,
    Checked: Future<Output = ()>,
{
    loop {
        check().await;
        tokio::select! {
            () = tokio::time::sleep(period) => {}
            () = restart.notified() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use tokio::sync::Notify;

    use super::repeat_checks;

    const PERIOD: Duration = Duration::from_secs(15 * 60);

    async fn settle() {
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn restart_checks_at_once_and_starts_the_period_again() {
        let checks = AtomicUsize::new(0);
        let restart = Notify::new();
        let count = || async { checks.fetch_add(1, Ordering::Relaxed); };
        let repeating = repeat_checks(PERIOD, &restart, count);
        let observe = async {
            settle().await;
            assert_eq!(checks.load(Ordering::Relaxed), 1);
            tokio::time::sleep(PERIOD).await;
            settle().await;
            assert_eq!(checks.load(Ordering::Relaxed), 2);
            tokio::time::sleep(PERIOD / 3).await;
            restart.notify_one();
            settle().await;
            assert_eq!(checks.load(Ordering::Relaxed), 3);
            tokio::time::sleep(PERIOD - Duration::from_secs(60)).await;
            settle().await;
            assert_eq!(checks.load(Ordering::Relaxed), 3);
            tokio::time::sleep(Duration::from_secs(61)).await;
            settle().await;
            assert_eq!(checks.load(Ordering::Relaxed), 4);
        };
        tokio::select! {
            () = repeating => unreachable!(),
            () = observe => {}
        }
    }
}
