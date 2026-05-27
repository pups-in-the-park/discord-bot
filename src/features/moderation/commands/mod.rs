//! Moderation slash commands, one per file. The `TimeoutDuration`/`DeleteMessages`
//! choice enums are shared by `timeout`/`ban`, so they live here.

mod ban;
mod history;
mod kick;
mod timeout;
mod unban;
mod untimeout;
mod warn;

pub use ban::ban;
pub use history::history;
pub use kick::kick;
pub use timeout::timeout;
pub use unban::unban;
pub use untimeout::untimeout;
pub use warn::warn;

#[derive(Debug, poise::ChoiceParameter)]
pub enum TimeoutDuration {
    #[name = "60 seconds"]
    Secs60,
    #[name = "5 minutes"]
    Mins5,
    #[name = "10 minutes"]
    Mins10,
    #[name = "1 hour"]
    Hour1,
    #[name = "1 day"]
    Day1,
    #[name = "1 week"]
    Week1,
}

impl TimeoutDuration {
    pub fn as_secs(&self) -> i64 {
        match self {
            Self::Secs60 => 60,
            Self::Mins5 => 300,
            Self::Mins10 => 600,
            Self::Hour1 => 3600,
            Self::Day1 => 86400,
            Self::Week1 => 604800,
        }
    }
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum DeleteMessages {
    #[name = "Don't delete any"]
    None,
    #[name = "Previous hour"]
    Hour,
    #[name = "Previous 6 hours"]
    Hours6,
    #[name = "Previous 24 hours"]
    Hours24,
    #[name = "Previous 3 days"]
    Days3,
    #[name = "Previous 7 days"]
    Days7,
}

impl DeleteMessages {
    pub fn as_secs(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Hour => 3600,
            Self::Hours6 => 21600,
            Self::Hours24 => 86400,
            Self::Days3 => 259200,
            Self::Days7 => 604800,
        }
    }
}
