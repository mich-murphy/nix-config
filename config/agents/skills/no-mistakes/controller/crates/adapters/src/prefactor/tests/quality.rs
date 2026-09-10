//! The quality payload against open and closed instances.

use super::*;

fn payload(judged: u32) -> domain::judge::QualityPayload {
    domain::judge::QualityPayload {
        tasks: BTreeMap::new(),
        rollup: domain::judge::Rollup {
            tasks: 2,
            judged,
            pass: judged,
            degraded: 0,
            fail: 0,
            mean_overall: 90,
            mean_friction: 80,
            friction_events: 1,
            avoidable_friction_events: 1,
            verdict: domain::judge::JudgeVerdict::Pass,
        },
    }
}

#[test]
fn quality_is_recorded_on_the_open_instance() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    let tracer = tracer(&fake);
    tracer.record(&mut state, &[record(1, completed()?)]);
    tracer.quality(&mut state, &payload(1));
    assert_eq!(
        fake.groups().last().map(String::as_str),
        Some("curl /agent_instance/i1/record_quality")
    );
    let calls = fake.calls.borrow();
    let (args, files) = calls.last().ok_or("no call")?;
    assert_eq!(args[1], "--fail-with-body");
    assert!(args.iter().all(|arg| !arg.contains("tok")));
    assert_eq!(files[0]["name"], "delivery-quality");
    assert_eq!(files[0]["payload"]["rollup"]["judged"], 1);
    Ok(())
}

#[test]
fn quality_after_finish_uses_the_closed_instance() -> Result<(), Box<dyn std::error::Error>> {
    let fake = Fake::new();
    let mut state = Memory::default();
    let tracer = tracer(&fake);
    tracer.record(&mut state, &[record(1, completed()?)]);
    tracer.finish(&mut state);
    assert_eq!(state.get(INSTANCE)?, None);
    tracer.quality(&mut state, &payload(2));
    assert_eq!(
        fake.groups().last().map(String::as_str),
        Some("curl /agent_instance/i1/record_quality")
    );
    Ok(())
}

#[test]
fn quality_without_any_instance_sends_nothing() {
    let fake = Fake::new();
    let mut state = Memory::default();
    tracer(&fake).quality(&mut state, &payload(0));
    assert!(fake.calls.borrow().is_empty());
}
