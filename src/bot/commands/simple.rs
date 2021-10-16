use crate::bot::qb_chat::MenuValue;
use itertools::Itertools;

use super::QbCommandAction;

pub struct QHelp {}

impl QbCommandAction for QHelp {
    fn action_result_to_string(&self) -> String {
        MenuValue::generate_cmds()
            .iter()
            .sorted()
            .map(|(cmd, menu_value)| format!("{} - {}", cmd, menu_value.get_help()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
