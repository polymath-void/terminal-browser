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
            if (handle.role == "button" || handle.role == "link")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pay_button_blocked_without_force() {
        let action = AgentAction {
            action_type: ActionType::Click,
            ref_id: Some(1),
            payload: None,
            force: false,
        };
        let pay_btn = AXNodeHandle {
            backend_node_id: 10,
            role: "button".to_string(),
            name: Some("Pay $50".to_string()),
        };
        assert_eq!(SecurityGate::classify_action(&action, Some(&pay_btn)), ActionRisk::Critical);
        let result = SecurityGate::can_execute(&action, Some(&pay_btn));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Critical financial or destructive action"));
    }

    #[test]
    fn test_pay_button_allowed_with_force() {
        let action = AgentAction {
            action_type: ActionType::Click,
            ref_id: Some(1),
            payload: None,
            force: true,
        };
        let pay_btn = AXNodeHandle {
            backend_node_id: 10,
            role: "button".to_string(),
            name: Some("Pay $50".to_string()),
        };
        assert_eq!(SecurityGate::classify_action(&action, Some(&pay_btn)), ActionRisk::Critical);
        let result = SecurityGate::can_execute(&action, Some(&pay_btn));
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_account_blocked_without_force() {
        let action = AgentAction {
            action_type: ActionType::Click,
            ref_id: Some(2),
            payload: None,
            force: false,
        };
        let del_btn = AXNodeHandle {
            backend_node_id: 20,
            role: "button".to_string(),
            name: Some("Delete Account".to_string()),
        };
        assert_eq!(SecurityGate::classify_action(&action, Some(&del_btn)), ActionRisk::Critical);
        let result = SecurityGate::can_execute(&action, Some(&del_btn));
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_account_allowed_with_force() {
        let action = AgentAction {
            action_type: ActionType::Click,
            ref_id: Some(2),
            payload: None,
            force: true,
        };
        let del_btn = AXNodeHandle {
            backend_node_id: 20,
            role: "button".to_string(),
            name: Some("Delete Account".to_string()),
        };
        assert_eq!(SecurityGate::classify_action(&action, Some(&del_btn)), ActionRisk::Critical);
        let result = SecurityGate::can_execute(&action, Some(&del_btn));
        assert!(result.is_ok());
    }

    #[test]
    fn test_destructive_and_financial_keywords() {
        let keywords = ["Confirm Purchase", "Transfer $1,000", "Pay invoice", "Delete profile"];
        for kw in &keywords {
            let action = AgentAction {
                action_type: ActionType::Click,
                ref_id: Some(3),
                payload: None,
                force: false,
            };
            let handle = AXNodeHandle {
                backend_node_id: 30,
                role: "button".to_string(),
                name: Some(kw.to_string()),
            };
            assert_eq!(SecurityGate::classify_action(&action, Some(&handle)), ActionRisk::Critical);
            assert!(SecurityGate::can_execute(&action, Some(&handle)).is_err());
        }
    }

    #[test]
    fn test_link_destructive_action_protection() {
        let action = AgentAction {
            action_type: ActionType::Click,
            ref_id: Some(4),
            payload: None,
            force: false,
        };
        let del_link = AXNodeHandle {
            backend_node_id: 40,
            role: "link".to_string(),
            name: Some("Delete Account Now".to_string()),
        };
        assert_eq!(SecurityGate::classify_action(&action, Some(&del_link)), ActionRisk::Critical);
        assert!(SecurityGate::can_execute(&action, Some(&del_link)).is_err());
    }

    #[test]
    fn test_elevated_risk_textbox() {
        let action = AgentAction {
            action_type: ActionType::Type,
            ref_id: Some(5),
            payload: None,
            force: false,
        };
        let pwd_box = AXNodeHandle {
            backend_node_id: 50,
            role: "textbox".to_string(),
            name: Some("Master Password".to_string()),
        };
        assert_eq!(SecurityGate::classify_action(&action, Some(&pwd_box)), ActionRisk::Elevated);
        assert!(SecurityGate::can_execute(&action, Some(&pwd_box)).is_ok());
    }

    #[test]
    fn test_safe_actions() {
        let nav_action = AgentAction {
            action_type: ActionType::Navigate,
            ref_id: None,
            payload: None,
            force: false,
        };
        assert_eq!(SecurityGate::classify_action(&nav_action, None), ActionRisk::Safe);
        assert!(SecurityGate::can_execute(&nav_action, None).is_ok());

        let click_action = AgentAction {
            action_type: ActionType::Click,
            ref_id: Some(6),
            payload: None,
            force: false,
        };
        let search_btn = AXNodeHandle {
            backend_node_id: 60,
            role: "button".to_string(),
            name: Some("Search DuckDuckGo".to_string()),
        };
        assert_eq!(SecurityGate::classify_action(&click_action, Some(&search_btn)), ActionRisk::Safe);
        assert!(SecurityGate::can_execute(&click_action, Some(&search_btn)).is_ok());
    }
}
