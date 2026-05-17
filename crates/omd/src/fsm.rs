use crate::types::*;

pub struct OmdFsm {
    agent: OmdAgent,
    phase: OmdPhase,
}

impl OmdFsm {
    pub fn new(agent: OmdAgent) -> Self {
        let phase = match agent {
            OmdAgent::Tongtian => OmdPhase::Tongtian(TongtianPhase::Explore),
            OmdAgent::Fuxi => OmdPhase::Fuxi(FuxiPhase::Interview),
            OmdAgent::Pangu => OmdPhase::Pangu(PanguPhase::LoadPlan),
            OmdAgent::Hongjun => OmdPhase::Hongjun(HongjunPhase::Intake),
        };
        Self { agent, phase }
    }

    pub fn agent(&self) -> OmdAgent { self.agent }
    pub fn phase(&self) -> &OmdPhase { &self.phase }
    pub fn current_phase_name(&self) -> &'static str { self.phase.name() }

    pub fn try_transition(&mut self, target: &str) -> Result<(), String> {
        if !self.all_phase_names().contains(&target) {
            return Err(format!(
                "'{}' is not a valid phase for {:?}. Valid: {:?}",
                target, self.agent, self.all_phase_names()
            ));
        }
        let valid = self.valid_next_phases();
        if !valid.contains(&target) {
            return Err(format!(
                "Cannot transition from '{}' to '{}'. Valid next phases: {:?}",
                self.current_phase_name(), target, valid
            ));
        }
        self.phase = self.resolve_phase(target);
        Ok(())
    }

    pub fn valid_next_phases(&self) -> Vec<&'static str> {
        match &self.phase {
            OmdPhase::Tongtian(p) => match p {
                TongtianPhase::Explore => vec!["Execute"],
                TongtianPhase::Execute => vec!["Verify"],
                TongtianPhase::Verify => vec!["Execute", "Done"],
                TongtianPhase::Done => vec![],
            },
            OmdPhase::Fuxi(p) => match p {
                FuxiPhase::Interview => vec!["Explore"],
                FuxiPhase::Explore => vec!["Architect"],
                FuxiPhase::Architect => vec!["Plan"],
                FuxiPhase::Plan => vec!["Done"],
                FuxiPhase::Done => vec![],
            },
            OmdPhase::Pangu(p) => match p {
                PanguPhase::LoadPlan => vec!["Decompose"],
                PanguPhase::Decompose => vec!["Delegate"],
                PanguPhase::Delegate => vec!["Verify"],
                PanguPhase::Verify => vec!["Delegate", "Done"],
                PanguPhase::Done => vec![],
            },
            OmdPhase::Hongjun(p) => match p {
                HongjunPhase::Intake => vec!["Route"],
                HongjunPhase::Route => vec!["Done"],
                HongjunPhase::Done => vec![],
            },
        }
    }

    /// Resume an FSM at a specific phase (for crash recovery / session resumption).
    pub fn with_phase(agent: OmdAgent, phase_name: &str) -> Result<Self, String> {
        let fsm = Self::new(agent);
        // If already at the target phase, nothing to do
        if fsm.current_phase_name() == phase_name {
            return Ok(fsm);
        }
        // Validate it's a real phase for this agent
        if !fsm.all_phase_names().contains(&phase_name) {
            return Err(format!("Phase '{}' is not valid for {:?}", phase_name, agent));
        }
        let phase = fsm.resolve_phase(phase_name);
        Ok(Self { agent, phase })
    }

    fn all_phase_names(&self) -> Vec<&'static str> {
        match self.agent {
            OmdAgent::Tongtian => vec!["Explore", "Execute", "Verify", "Done"],
            OmdAgent::Fuxi => vec!["Interview", "Explore", "Architect", "Plan", "Done"],
            OmdAgent::Pangu => vec!["LoadPlan", "Decompose", "Delegate", "Verify", "Done"],
            OmdAgent::Hongjun => vec!["Intake", "Route", "Done"],
        }
    }

    fn resolve_phase(&self, name: &str) -> OmdPhase {
        match self.agent {
            OmdAgent::Tongtian => OmdPhase::Tongtian(match name {
                "Explore" => TongtianPhase::Explore,
                "Execute" => TongtianPhase::Execute,
                "Verify" => TongtianPhase::Verify,
                _ => TongtianPhase::Done,
            }),
            OmdAgent::Fuxi => OmdPhase::Fuxi(match name {
                "Interview" => FuxiPhase::Interview,
                "Explore" => FuxiPhase::Explore,
                "Architect" => FuxiPhase::Architect,
                "Plan" => FuxiPhase::Plan,
                _ => FuxiPhase::Done,
            }),
            OmdAgent::Pangu => OmdPhase::Pangu(match name {
                "LoadPlan" => PanguPhase::LoadPlan,
                "Decompose" => PanguPhase::Decompose,
                "Delegate" => PanguPhase::Delegate,
                "Verify" => PanguPhase::Verify,
                _ => PanguPhase::Done,
            }),
            OmdAgent::Hongjun => OmdPhase::Hongjun(match name {
                "Intake" => HongjunPhase::Intake,
                "Route" => HongjunPhase::Route,
                _ => HongjunPhase::Done,
            }),
        }
    }
}
