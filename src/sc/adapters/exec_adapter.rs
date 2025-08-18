use crossbeam_channel::{ bounded, select, Receiver as CbReceiver };
use streamdeck_lib::prelude::*;
use std::{ sync::Arc, time::Duration };

use crate::{ bindings::action_bindings::ActionBindingsStore, sc::topics::ExecSend };
use crate::sc::topics::EXEC_SEND;

pub struct ExecAdapter;

impl ExecAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl AdapterStatic for ExecAdapter {
    const NAME: &'static str = "sc.exec_adapter";
}

impl Adapter for ExecAdapter {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn policy(&self) -> StartPolicy {
        StartPolicy::OnAppLaunch
    }

    fn topics(&self) -> &'static [&'static str] {
        &[EXEC_SEND.name]
    }

    fn start(
        &self,
        cx: &Context,
        _bus: Arc<dyn Bus>,
        inbox: CbReceiver<Arc<ErasedTopic>>
    ) -> AdapterResult {
        let (stop_tx, stop_rx) = bounded::<()>(1);
        let logger = cx.log().clone();
        let store = cx
            .try_ext::<ActionBindingsStore>()
            .ok_or(AdapterError::Init("ActionBindingsStore extension missing".to_string()))?
            .clone();

        let join = std::thread::spawn(move || {
            info!(logger, "ExecAdapter started");
            loop {
                select! {
                    recv(inbox) -> msg => match msg {
                        Ok(ev) => {
                            let Some(m) = ev.downcast(EXEC_SEND) else { continue };
                            debug!(logger, "recv: {:?}", m);
                            if let Err(e) = handle_exec(&store, &logger, m) {
                                warn!(logger, "exec: {}", e);
                            }
                        }
                        Err(e) => error!(logger, "recv: {}", e),
                    },
                    recv(stop_rx) -> _ => break,
                    default(Duration::from_millis(50)) => {},
                }
            }
            info!(logger, "ExecAdapter stopped");
        });

        Ok(AdapterHandle::from_crossbeam(join, stop_tx))
    }
}

fn handle_exec(
    store: &ActionBindingsStore,
    logger: &Arc<dyn ActionLog>,
    msg: &ExecSend
) -> Result<(), String> {
    let action = store
        .get_binding_by_id(&msg.action_id)
        .ok_or_else(|| format!("action '{}' not found", msg.action_id))?;

    let hold_ms = msg.hold_ms.map(Duration::from_millis);
    let bindings = store.snapshot();

    action
        .simulate_using(Arc::clone(logger), hold_ms, msg.is_down, &bindings)
        .map_err(|e| format!("simulate: {e}"))
}
