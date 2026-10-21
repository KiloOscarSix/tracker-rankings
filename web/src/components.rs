#![expect(
    unreachable_pub,
    reason = "registry sources are kept as `topcoat ui` installs them"
)]

pub mod alert_dialog;
pub mod badge;
pub mod button;
pub mod confirm;
pub mod dialog;
pub mod label;
pub mod radio_group;
pub mod score;
pub mod table;
