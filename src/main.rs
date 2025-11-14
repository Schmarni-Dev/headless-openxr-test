use std::{ptr, thread, time::Duration};

use openxr::{
    ActiveActionSet, Binding, ExtensionSet, FormFactor, FrameStream, FrameWaiter, Session,
    SessionState, SystemId, Vector2f, ViewConfigurationType, Vulkan, sys,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    info!("Hewow oworld");
    let oxr_entry = unsafe { openxr::Entry::load() }.unwrap();
    let exts = oxr_entry.enumerate_extensions().unwrap();
    let mut enabled_exts = ExtensionSet::default();
    if !exts.mnd_headless {
        panic!("no headless");
    }
    enabled_exts.mnd_headless = true;
    let oxr_instance = oxr_entry
        .create_instance(
            &openxr::ApplicationInfo {
                application_name: "headless test",
                application_version: 1,
                engine_name: "nothing",
                engine_version: 1,
                api_version: openxr::Version::new(1, 1, 0),
            },
            &enabled_exts,
            &[],
        )
        .unwrap();
    let system = oxr_instance
        .system(FormFactor::HEAD_MOUNTED_DISPLAY)
        .unwrap();
    let system_props = oxr_instance.system_properties(system).unwrap();
    info!(?system_props);
    let (oxr_session, mut oxr_frame_waiter, _oxr_frame_stream) =
        create_session(&oxr_instance, system).unwrap();

    let mut session_running = false;
    info!("create buffer");
    let action_set = oxr_instance.create_action_set("set", "set", 100).unwrap();
    let action = action_set
        .create_action::<Vector2f>("action", "action", &[])
        .unwrap();
    oxr_instance
        .suggest_interaction_profile_bindings(
            oxr_instance
                .string_to_path("/interaction_profiles/valve/index_controller")
                .unwrap(),
            &[
                Binding::new(
                    &action,
                    oxr_instance
                        .string_to_path("/user/hand/left/input/thumbstick")
                        .unwrap(),
                ),
                Binding::new(
                    &action,
                    oxr_instance
                        .string_to_path("/user/hand/right/input/thumbstick")
                        .unwrap(),
                ),
            ],
        )
        .unwrap();
    oxr_instance
        .suggest_interaction_profile_bindings(
            oxr_instance
                .string_to_path("/interaction_profiles/oculus/touch_controller")
                .unwrap(),
            &[
                Binding::new(
                    &action,
                    oxr_instance
                        .string_to_path("/user/hand/left/input/thumbstick")
                        .unwrap(),
                ),
                Binding::new(
                    &action,
                    oxr_instance
                        .string_to_path("/user/hand/right/input/thumbstick")
                        .unwrap(),
                ),
            ],
        )
        .unwrap();

    oxr_session.attach_action_sets(&[&action_set]).unwrap();

    loop {
        let mut event_buffer = openxr::EventDataBuffer::new();
        while let Some(e) = oxr_instance.poll_event(&mut event_buffer).unwrap() {
            match e {
                openxr::Event::EventsLost(_events_lost) => todo!(),
                openxr::Event::InstanceLossPending(_instance_loss_pending) => todo!(),
                openxr::Event::SessionStateChanged(session_state_changed) => {
                    match dbg!(session_state_changed.state()) {
                        SessionState::READY => {
                            oxr_session
                                .begin(ViewConfigurationType::PRIMARY_STEREO)
                                .unwrap();
                            session_running = true;
                        }
                        SessionState::STOPPING
                        | SessionState::LOSS_PENDING
                        | SessionState::IDLE
                        | SessionState::EXITING => session_running = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if !session_running {
            thread::sleep(Duration::from_millis(5));
            continue;
        }
        let _state = oxr_frame_waiter.wait().unwrap();
        oxr_session
            .sync_actions(&[ActiveActionSet::new(&action_set)])
            .unwrap();
        let v = action.state(&oxr_session, openxr::Path::NULL).unwrap();
        let v = v.current_state;
        info!("action value: {:?}", v);
    }
}

fn create_session(
    instance: &openxr::Instance,
    system: SystemId,
    // this isn't actually a vulkan session, but the crate doesn't support headless properly
) -> openxr::Result<(Session<Vulkan>, FrameWaiter, FrameStream<Vulkan>)> {
    let raw = create_raw_session(instance, system)?;
    unsafe { Ok(Session::from_raw(instance.clone(), raw, Box::new(()))) }
}

fn create_raw_session(
    instance: &openxr::Instance,
    system: SystemId,
) -> openxr::Result<sys::Session> {
    let info = sys::SessionCreateInfo {
        ty: sys::SessionCreateInfo::TYPE,
        next: ptr::null(),
        create_flags: Default::default(),
        system_id: system,
    };
    let mut out = sys::Session::NULL;
    unsafe {
        cvt((instance.fp().create_session)(
            instance.as_raw(),
            &info,
            &mut out,
        ))?;
    }
    Ok(out)
}

// FFI helpers
fn cvt(x: sys::Result) -> openxr::Result<sys::Result> {
    if x.into_raw() >= 0 { Ok(x) } else { Err(x) }
}
