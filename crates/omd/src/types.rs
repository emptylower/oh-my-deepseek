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
