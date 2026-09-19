use std::sync::mpsc::{Receiver, Sender};

use super::{HelpRequest, HelpResponse, discover};

pub(super) fn run(requests: Receiver<HelpRequest>, responses: Sender<HelpResponse>) {
    loop {
        let request = match requests.recv().ok() {
            Some(request) => request,
            None => return,
        };
        let request = drain_latest(request, &requests);
        let started = std::time::Instant::now();
        let response_request = request.request.clone();
        let spec = discover::load(&request);
        if responses
            .send(HelpResponse {
                key: request.key.clone(),
                request: response_request,
                spec,
                elapsed: started.elapsed(),
            })
            .is_err()
        {
            return;
        }
    }
}

fn drain_latest(mut request: HelpRequest, requests: &Receiver<HelpRequest>) -> HelpRequest {
    while let Ok(next) = requests.try_recv() {
        request = next;
    }
    request
}
