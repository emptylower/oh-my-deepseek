use omd::fsm::OmdFsm;
use omd::types::OmdAgent;

#[test]
fn tongtian_valid_forward_transitions() {
    let mut fsm = OmdFsm::new(OmdAgent::Tongtian);
    assert_eq!(fsm.current_phase_name(), "Explore");
    assert!(fsm.try_transition("Execute").is_ok());
    assert_eq!(fsm.current_phase_name(), "Execute");
    assert!(fsm.try_transition("Verify").is_ok());
    assert_eq!(fsm.current_phase_name(), "Verify");
    assert!(fsm.try_transition("Done").is_ok());
    assert_eq!(fsm.current_phase_name(), "Done");
}

#[test]
fn tongtian_verify_loops_back_to_execute() {
    let mut fsm = OmdFsm::new(OmdAgent::Tongtian);
    fsm.try_transition("Execute").unwrap();
    fsm.try_transition("Verify").unwrap();
    assert!(fsm.try_transition("Execute").is_ok());
    assert_eq!(fsm.current_phase_name(), "Execute");
}

#[test]
fn tongtian_rejects_invalid_skip() {
    let mut fsm = OmdFsm::new(OmdAgent::Tongtian);
    let err = fsm.try_transition("Done").unwrap_err();
    assert!(err.contains("Cannot transition"));
    assert!(err.contains("Execute"));
}

#[test]
fn tongtian_rejects_unknown_phase() {
    let mut fsm = OmdFsm::new(OmdAgent::Tongtian);
    let err = fsm.try_transition("interview").unwrap_err();
    assert!(err.contains("not a valid phase"));
}

#[test]
fn from_phase_logged_before_transition() {
    let mut fsm = OmdFsm::new(OmdAgent::Tongtian);
    let from = fsm.current_phase_name().to_string();
    fsm.try_transition("Execute").unwrap();
    assert_eq!(from, "Explore");
    assert_eq!(fsm.current_phase_name(), "Execute");
}
