use core::fmt::Debug;

use crate::Result;
// use alloc::boxed::Box;
pub(crate) use tinywasm_types::{ResumeArgument, YieldedValue};

///"coroutine statse", "coroutine instance", "resumable". Stores info to continue a function that was paused
pub trait CoroState<Ret, ResumeContext>: Debug {
    /// resumes the execution of the coroutine
    fn resume(&mut self, ctx: ResumeContext, arg: ResumeArgument) -> Result<CoroStateResumeResult<Ret>>;
}

/// explains why did execution suspend, and carries payload if needed
#[derive(Debug)]
pub enum SuspendReason {
    /// host function yielded
    /// potentially some host functions might expect resume argument when calling resume
    Yield(YieldedValue),

    /// time to suspend has come,
    /// host shouldn't provide resume argument when calling resume
    #[cfg(feature = "std")]
    SuspendedEpoch,

    /// user's should-suspend-callback,
    /// host shouldn't provide resume argument when calling resume
    SuspendedCallback,

    /// async should_suspend flag was set
    /// host shouldn't provide resume argument when calling resume
    SuspendedFlag,
    // possible others: delimited continuations proposal, debugger breakpoint, out of fuel
}

/// result of a function that might pause in the middle and yield
/// to be resumed later
#[derive(Debug)]
pub enum PotentialCoroCallResult<R, State>
//where for<Ctx>
//    State: CoroState<R, Ctx>, // can't in stable rust
{
    /// function returns normally
    Return(R),
    /// interpreter will be suspended and execution will return to host along with SuspendReason
    Suspended(SuspendReason, State),
}

/// result of resuming coroutine state. Unlike [`PotentialCoroCallResult`]
/// doesn't need to have state, since it's contained in self
#[derive(Debug)]
pub enum CoroStateResumeResult<R> {
    /// CoroState has finished
    /// after this CoroState::resume can't be called again on that CoroState
    Return(R),

    /// host function yielded
    /// execution returns to host along with yielded value
    Suspended(SuspendReason),
}

impl<R, State> PotentialCoroCallResult<R, State> {

    /// in case you expect function only to return
    /// you can make Suspend into [crate::Error::UnexpectedSuspend] error
    pub fn suspend_to_err(self) -> Result<R> {
        match self {
            PotentialCoroCallResult::Return(r) => Ok(r),
            PotentialCoroCallResult::Suspended(r, _) => Err(crate::Error::UnexpectedSuspend(r)),
        }
    }

    /// true if coro is finished
    pub fn finished(&self) -> bool {
        matches!(self, Self::Return(_))
    }
    /// separates state from PotentialCoroCallResult, leaving CoroStateResumeResult (one without state)
    pub fn split_state(self) -> (CoroStateResumeResult<R>, Option<State>) {
        match self {
            Self::Return(val) => (CoroStateResumeResult::Return(val), None),
            Self::Suspended(suspend, state) => (CoroStateResumeResult::Suspended(suspend), Some(state)),
        }
    }
    /// separates result from PotentialCoroCallResult, leaving unit type in it's place
    pub fn split_result(self) -> (PotentialCoroCallResult<(), State>, Option<R>) {
        match self {
            Self::Return(result) => (PotentialCoroCallResult::Return(()), Some(result)),
            Self::Suspended(suspend, state) => (PotentialCoroCallResult::Suspended(suspend, state), None),
        }
    }

    /// transforms state
    pub fn map_state<OutS>(self, mapper: impl FnOnce(State) -> OutS) -> PotentialCoroCallResult<R, OutS> {
        match self {
            Self::Return(val) => PotentialCoroCallResult::Return(val),
            Self::Suspended(suspend, state) => PotentialCoroCallResult::Suspended(suspend, mapper(state)),
        }
    }
    /// transform result with mapper if there is none - calls "otherwise" arg. user_val
    pub fn map_result_or_else<OutR, Usr>(
        self,
        user_val: Usr,
        mapper: impl FnOnce(R, Usr) -> OutR,
        otherwise: impl FnOnce(Usr),
    ) -> PotentialCoroCallResult<OutR, State> {
        match self {
            Self::Return(res) => PotentialCoroCallResult::Return(mapper(res, user_val)),
            Self::Suspended(suspend, state) => {
                otherwise(user_val);
                PotentialCoroCallResult::Suspended(suspend, state)
            }
        }
    }
    /// transforms result
    pub fn map_result<OutR>(self, mapper: impl FnOnce(R) -> OutR) -> PotentialCoroCallResult<OutR, State> {
        self.map_result_or_else((), |val, _| mapper(val), |_| {})
    }
}

impl<R> CoroStateResumeResult<R> {
    /// true if coro is finished
    pub fn finished(&self) -> bool {
        matches!(self, Self::Return(_))
    }
    /// separates result from CoroStateResumeResult, leaving unit type in it's place
    pub fn split_result(self) -> (CoroStateResumeResult<()>, Option<R>) {
        let (a, r) = PotentialCoroCallResult::<R, ()>::from(self).split_result();
        (a.into(), r)
    }
    /// transforms result
    pub fn map_result<OutR>(self, mapper: impl FnOnce(R) -> OutR) -> CoroStateResumeResult<OutR> {
        PotentialCoroCallResult::<R, ()>::from(self).map_result(mapper).into()
    }
    /// transform result with mapper if there is none - calls "otherwise" arg. user_val called
    pub fn map_result_or_else<OutR, Usr>(
        self,
        user_val: Usr,
        mapper: impl FnOnce(R, Usr) -> OutR,
        otherwise: impl FnOnce(Usr),
    ) -> CoroStateResumeResult<OutR> {
        PotentialCoroCallResult::<R, ()>::from(self).map_result_or_else(user_val, mapper, otherwise).into()
    }
}

impl<DstR, SrcR> From<PotentialCoroCallResult<SrcR, ()>> for CoroStateResumeResult<DstR>
where
    DstR: From<SrcR>,
{
    fn from(value: PotentialCoroCallResult<SrcR, ()>) -> Self {
        match value {
            PotentialCoroCallResult::Return(val) => Self::Return(val.into()),
            PotentialCoroCallResult::Suspended(suspend, ()) => Self::Suspended(suspend),
        }
    }
}
impl<SrcR> From<CoroStateResumeResult<SrcR>> for PotentialCoroCallResult<SrcR, ()> {
    fn from(value: CoroStateResumeResult<SrcR>) -> Self {
        match value {
            CoroStateResumeResult::Return(val) => PotentialCoroCallResult::Return(val),
            CoroStateResumeResult::Suspended(suspend) => PotentialCoroCallResult::Suspended(suspend, ()),
        }
    }
}
