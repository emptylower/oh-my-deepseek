use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OmdAgent {
    Hongjun,
    Fuxi,
    Pangu,
    Tongtian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TongtianPhase { Explore, Execute, Verify, Done }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FuxiPhase { Interview, Explore, Architect, Plan, Done }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanguPhase { LoadPlan, Decompose, Delegate, Verify, Done }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HongjunPhase { Intake, Route, Done }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OmdPhase {
    Tongtian(TongtianPhase),
    Fuxi(FuxiPhase),
    Pangu(PanguPhase),
    Hongjun(HongjunPhase),
}

impl OmdPhase {
    /// Resolve agent name + phase name strings into an `OmdPhase`.
    /// Returns `None` if the combination is invalid.
    pub fn from_agent_and_name(agent: &str, phase: &str) -> Option<Self> {
        match agent {
            "Tongtian" => match phase {
                "Explore" => Some(Self::Tongtian(TongtianPhase::Explore)),
                "Execute" => Some(Self::Tongtian(TongtianPhase::Execute)),
                "Verify" => Some(Self::Tongtian(TongtianPhase::Verify)),
                "Done" => Some(Self::Tongtian(TongtianPhase::Done)),
                _ => None,
            },
            "Fuxi" => match phase {
                "Interview" => Some(Self::Fuxi(FuxiPhase::Interview)),
                "Explore" => Some(Self::Fuxi(FuxiPhase::Explore)),
                "Architect" => Some(Self::Fuxi(FuxiPhase::Architect)),
                "Plan" => Some(Self::Fuxi(FuxiPhase::Plan)),
                "Done" => Some(Self::Fuxi(FuxiPhase::Done)),
                _ => None,
            },
            "Pangu" => match phase {
                "LoadPlan" => Some(Self::Pangu(PanguPhase::LoadPlan)),
                "Decompose" => Some(Self::Pangu(PanguPhase::Decompose)),
                "Delegate" => Some(Self::Pangu(PanguPhase::Delegate)),
                "Verify" => Some(Self::Pangu(PanguPhase::Verify)),
                "Done" => Some(Self::Pangu(PanguPhase::Done)),
                _ => None,
            },
            "Hongjun" => match phase {
                "Intake" => Some(Self::Hongjun(HongjunPhase::Intake)),
                "Route" => Some(Self::Hongjun(HongjunPhase::Route)),
                "Done" => Some(Self::Hongjun(HongjunPhase::Done)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Tongtian(p) => match p {
                TongtianPhase::Explore => "Explore",
                TongtianPhase::Execute => "Execute",
                TongtianPhase::Verify => "Verify",
                TongtianPhase::Done => "Done",
            },
            Self::Fuxi(p) => match p {
                FuxiPhase::Interview => "Interview",
                FuxiPhase::Explore => "Explore",
                FuxiPhase::Architect => "Architect",
                FuxiPhase::Plan => "Plan",
                FuxiPhase::Done => "Done",
            },
            Self::Pangu(p) => match p {
                PanguPhase::LoadPlan => "LoadPlan",
                PanguPhase::Decompose => "Decompose",
                PanguPhase::Delegate => "Delegate",
                PanguPhase::Verify => "Verify",
                PanguPhase::Done => "Done",
            },
            Self::Hongjun(p) => match p {
                HongjunPhase::Intake => "Intake",
                HongjunPhase::Route => "Route",
                HongjunPhase::Done => "Done",
            },
        }
    }
}
