use super::*;

pub(super) fn apply(s: &mut State, input: Input, at: u64) -> Result<Value> {
    match input {
        input @ Input::ConfigureLoop { .. } => handle_configure_loop(s, input, at),
        input @ Input::LessonRecord { .. } => handle_lesson_record(s, input, at),
        input @ Input::LoopPlan { .. } => handle_loop_plan(s, input, at),
        input @ Input::Measure { .. } => handle_measure(s, input, at),
        input @ Input::Checkpoint { .. } => handle_checkpoint(s, input, at),
        input @ Input::HumanReview { .. } => handle_human_review(s, input, at),
        _ => bail!("incorrect command group"),
    }
}

fn handle_configure_loop(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::ConfigureLoop { policy } = input else {
        bail!("incorrect command routed to handle_configure_loop");
    };
    loops::configure(s, policy)
}

fn handle_lesson_record(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::LessonRecord { key, lesson } = input else {
        bail!("incorrect command routed to handle_lesson_record");
    };
    loops::record_lesson(s, &key, lesson)
}

fn handle_loop_plan(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::LoopPlan { key, plan } = input else {
        bail!("incorrect command routed to handle_loop_plan");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    let result = loops::plan(s, &mut t, plan)?;
    if !t
        .loop_state
        .plan
        .as_ref()
        .unwrap()
        .input
        .lesson_families
        .is_empty()
    {
        s.version = s.version.max(6);
    }
    save(s, &key, t);
    Ok(result)
}

fn handle_measure(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::Measure {
        key,
        criterion,
        observed,
        satisfied,
        artifact,
    } = input
    else {
        bail!("incorrect command routed to handle_measure");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    let result = loops::measure(s, &mut t, criterion, observed, satisfied, artifact)?;
    save(s, &key, t);
    Ok(result)
}

fn handle_checkpoint(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::Checkpoint {
        key,
        advanced,
        finding,
        next_step,
        artifact,
        scope_reason,
    } = input
    else {
        bail!("incorrect command routed to handle_checkpoint");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    let checkpoint = loops::Checkpoint {
        launch: None,
        snapshot: fresh(s, &t)?,
        advanced,
        finding,
        next_step,
        digest: crate::store::artifact(&artifact)?.parse()?,
        artifact,
        scope_reason,
        size: Default::default(),
        paths: vec![],
        outside_plan: vec![],
    };
    let result = loops::checkpoint(s, &mut t, checkpoint)?;
    save(s, &key, t);
    Ok(result)
}

fn handle_human_review(s: &mut State, input: Input, _at: u64) -> Result<Value> {
    let Input::HumanReview {
        key,
        snapshot,
        source,
        artifact,
    } = input
    else {
        bail!("incorrect command routed to handle_human_review");
    };

    idle(s)?;
    let mut t = active(s, &key)?;
    let result = loops::record_human(s, &mut t, snapshot, source, artifact)?;
    save(s, &key, t);
    Ok(result)
}
