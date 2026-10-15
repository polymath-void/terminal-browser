use term_core::{AXNodeHandle, ActionType, AgentAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionRisk {
    Safe,
    Elevated,
    Critical,
}

pub struct SecurityGate;

impl SecurityGate {
    pub fn classify_action(action: &AgentAction, element: Option<&AXNodeHandle>) -> ActionRisk {
        if action.action_type == ActionType::Navigate || action.action_type == ActionType::Scroll {
            return ActionRisk::Safe;
        }

        if let Some(handle) = element {
            let name_lower = handle.name.as_deref().unwrap_or("").to_lowercase();
            if handle.role == "button"
                && (name_lower.contains("pay")
                    || name_lower.contains("delete")
                    || name_lower.contains("confirm purchase")
                    || name_lower.contains("transfer"))
            {
                return ActionRisk::Critical;
            }
            if handle.role == "textbox" && (name_lower.contains("password") || name_lower.contains("credit card")) {
                return ActionRisk::Elevated;
            }
        }

        ActionRisk::Safe
    }

    pub fn can_execute(action: &AgentAction, element: Option<&AXNodeHandle>) -> Result<(), String> {
        let risk = Self::classify_action(action, element);
        if risk == ActionRisk::Critical && !action.force {
            return Err("Action paused: Critical financial or destructive action requires human confirmation (or force=true)".to_string());
        }
        Ok(())
    }
}
