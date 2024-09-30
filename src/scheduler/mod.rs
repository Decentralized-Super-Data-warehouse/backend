use std::sync::Arc;
use tokio::time::{interval, sleep, Duration};

use crate::{database::PostgreDatabase, external::External};

#[derive(Clone)]
enum TaskType {
    TotalValueLocked,
    TokenTerminalData,
    MarketCap,
    NumberOfTokenHolders,
    TradingVolume,
    DailyActiveUsers,
    WeeklyActiveUsers,
    DailyFees,
}

pub struct Task {
    interval: Duration,
    project_id: i32,
    task_type: TaskType,
}

pub struct Scheduler {
    db: Arc<PostgreDatabase>,
    external: Arc<External>,
}

impl Scheduler {
    pub fn new(db: PostgreDatabase, external: External) -> Self {
        Scheduler {
            db: Arc::new(db),
            external: Arc::new(external),
        }
    }

    pub async fn run_task(task: Task, db: Arc<PostgreDatabase>, external: Arc<External>) {
        let mut interval = interval(task.interval);
        loop {
            interval.tick().await;
            match task.task_type {
                TaskType::TotalValueLocked => {
                    Self::update_total_value_locked(&db, &external, task.project_id).await
                }
                TaskType::TokenTerminalData => {
                    Self::update_token_terminal_data(&db, &external, task.project_id).await
                }
                TaskType::MarketCap => {
                    Self::update_market_cap(&db, &external, task.project_id).await
                }
                TaskType::NumberOfTokenHolders => {
                    Self::update_number_of_token_holders(&db, &external, task.project_id).await
                }
                TaskType::TradingVolume => {
                    Self::update_trading_volume(&db, &external, task.project_id).await
                }
                TaskType::DailyActiveUsers => {
                    Self::update_daily_active_users(&db, &external, task.project_id).await
                }
                TaskType::WeeklyActiveUsers => {
                    Self::update_weekly_active_users(&db, &external, task.project_id).await
                }
                TaskType::DailyFees => {
                    Self::update_daily_fees(&db, &external, task.project_id).await
                }
            }
        }
    }

    pub async fn spawn_tasks(&self) {
        let tasks = vec![
            Task {
                interval: Duration::from_secs(3600),
                project_id: 1,
                task_type: TaskType::TotalValueLocked,
            },
            Task {
                interval: Duration::from_secs(240),
                project_id: 1,
                task_type: TaskType::TokenTerminalData,
            },
            Task {
                interval: Duration::from_secs(3600),
                project_id: 1,
                task_type: TaskType::MarketCap,
            },
            Task {
                interval: Duration::from_secs(86400),
                project_id: 1,
                task_type: TaskType::NumberOfTokenHolders,
            },
            Task {
                interval: Duration::from_secs(3600),
                project_id: 1,
                task_type: TaskType::TradingVolume,
            },
            Task {
                interval: Duration::from_secs(7200),
                project_id: 1,
                task_type: TaskType::DailyActiveUsers,
            },
            Task {
                interval: Duration::from_secs(86400),
                project_id: 1,
                task_type: TaskType::WeeklyActiveUsers,
            },
            Task {
                interval: Duration::from_secs(86400),
                project_id: 1,
                task_type: TaskType::DailyFees,
            },
        ];

        let delay = Duration::from_secs(120);
        for task in tasks {
            let db = Arc::clone(&self.db);
            let external = Arc::clone(&self.external);
            tokio::spawn(async move {
                Self::run_task(task, db, external).await;
            });
            sleep(delay).await;
        }
    }

    async fn update_total_value_locked(db: &PostgreDatabase, external: &External, project_id: i32) {
        match external
            .get_total_value_locked(
                "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa",
            )
            .await
        {
            Ok(tvl) => {
                if let Err(e) = db
                    .update_project_attribute(project_id, "total_value_locked", tvl.to_string())
                    .await
                {
                    eprintln!("Error updating TVL: {}", e);
                }
            }
            Err(e) => eprintln!("Error getting TVL: {}", e),
        }
    }

    async fn update_token_terminal_data(
        db: &PostgreDatabase,
        external: &External,
        project_id: i32,
    ) {
        match external.get_data_from_tokenterminal("pancakeswap").await {
            Ok(data) => {
                let updates = vec![
                    ("ath", data.ath),
                    ("ath_last", data.ath_last),
                    ("atl", data.atl),
                    ("atl_last", data.atl_last),
                    ("revenue_30d", data.revenue_30d),
                    ("revenue_annualized", data.revenue_annualized),
                    ("expenses_30d", data.expenses_30d),
                    ("earnings_30d", data.earnings_30d),
                    ("fees_30d", data.fees_30d),
                    ("fees_annualized", data.fees_annualized),
                    ("token_incentives_30d", data.token_incentives_30d),
                    ("monthly_active_users", data.monthly_active_users),
                    ("afpu", data.afpu),
                    ("arpu", data.arpu),
                    ("token_trading_volume_30d", data.token_trading_volume_30d),
                ];
                for (key, value) in updates {
                    if let Err(e) = db.update_project_attribute(project_id, key, value).await {
                        eprintln!("Error updating {}: {}", key, e);
                    }
                }
            }
            Err(e) => eprintln!("Error getting token terminal data: {}", e),
        }
    }

    async fn update_market_cap(db: &PostgreDatabase, external: &External, project_id: i32) {
        match external
            .calculate_market_cap(
                db,
                "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa",
                "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6::oft::CakeOFT",
                "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6",
            )
            .await
        {
            Ok(market_cap) => {
                if let Err(e) = db
                    .update_project_attribute(
                        project_id,
                        "market_cap_fully_diluted",
                        market_cap.fully_diluted.to_string(),
                    )
                    .await
                {
                    eprintln!("Error updating fully diluted market cap: {}", e);
                }
                if let Err(e) = db
                    .update_project_attribute(
                        project_id,
                        "market_cap_circulating",
                        market_cap.normal.to_string(),
                    )
                    .await
                {
                    eprintln!("Error updating circulating market cap: {}", e);
                }
            }
            Err(e) => eprintln!("Error calculating market cap: {}", e),
        }
    }

    async fn update_number_of_token_holders(
        db: &PostgreDatabase,
        external: &External,
        project_id: i32,
    ) {
        match external
            .get_number_of_token_holders(
                "0x159df6b7689437016108a019fd5bef736bac692b6d4a1f10c941f6fbb9a74ca6::oft::CakeOFT",
            )
            .await
        {
            Ok(holders) => {
                if let Err(e) = db
                    .update_project_attribute(project_id, "num_token_holders", holders.to_string())
                    .await
                {
                    eprintln!("Error updating number of token holders: {}", e);
                }
            }
            Err(e) => eprintln!("Error getting number of token holders: {}", e),
        }
    }

    async fn update_trading_volume(db: &PostgreDatabase, external: &External, project_id: i32) {
        match external.calculate_trading_volume(
            "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa",
            "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa::router::swap_exact_input"
        ).await {
            Ok(volume) => {
                if let Err(e) = db.update_project_attribute(project_id, "trading_volume", volume.to_string()).await {
                    eprintln!("Error updating trading volume: {}", e);
                }
            },
            Err(e) => eprintln!("Error calculating trading volume: {}", e),
        }
    }

    async fn update_daily_active_users(db: &PostgreDatabase, external: &External, project_id: i32) {
        match external
            .get_daily_active_users(
                "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa",
            )
            .await
        {
            Ok(users) => {
                if let Err(e) = db
                    .update_project_attribute(project_id, "daily_active_users", users.to_string())
                    .await
                {
                    eprintln!("Error updating daily active users: {}", e);
                }
            }
            Err(e) => eprintln!("Error getting daily active users: {}", e),
        }
    }

    async fn update_weekly_active_users(
        db: &PostgreDatabase,
        external: &External,
        project_id: i32,
    ) {
        match external
            .get_weekly_active_users(
                "0xc7efb4076dbe143cbcd98cfaaa929ecfc8f299203dfff63b95ccb6bfe19850fa",
            )
            .await
        {
            Ok(users) => {
                if let Err(e) = db
                    .update_project_attribute(project_id, "weekly_active_users", users.to_string())
                    .await
                {
                    eprintln!("Error updating weekly active users: {}", e);
                }
            }
            Err(e) => eprintln!("Error getting weekly active users: {}", e),
        }
    }

    async fn update_daily_fees(db: &PostgreDatabase, external: &External, project_id: i32) {
        match external.get_fee_within_n_days_pancake(1).await {
            Ok(fees) => {
                if let Err(e) = db
                    .update_project_attribute(project_id, "daily_fees", fees.to_string())
                    .await
                {
                    eprintln!("Error updating daily fees: {}", e);
                }
            }
            Err(e) => eprintln!("Error getting daily fees: {}", e),
        }
    }
}
