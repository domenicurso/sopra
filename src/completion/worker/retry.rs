use std::{
    sync::mpsc::{Receiver, Sender},
    time::{Duration, Instant},
};

use crate::completion::{Completion, CompletionResponse, Request};

use super::{complete_with_budget, send_response};

const ATTEMPT_BUDGET: Duration = Duration::from_millis(500);
const RETRY_BUDGET: Duration = Duration::from_secs(2);

pub(super) fn request(
    requests: &Receiver<Request>,
    responses: &Sender<CompletionResponse>,
    request: &Request,
    initial_items: &[Completion],
    request_started: Instant,
) -> Option<Request> {
    let deadline = Instant::now() + RETRY_BUDGET;
    let mut previous_items = initial_items.to_vec();
    while Instant::now() < deadline {
        if let Some(next) = newest_request(requests) {
            return Some(next);
        }
        std::thread::sleep(Duration::from_millis(25));
        let (items, incomplete) = complete_with_budget(request, ATTEMPT_BUDGET);
        if (!items.is_empty() && items != previous_items) || !incomplete {
            if !send_response(
                responses,
                request,
                items.clone(),
                incomplete,
                request_started.elapsed(),
            ) {
                return None;
            }
            previous_items = items;
        }
        if !incomplete {
            return None;
        }
    }
    None
}

fn newest_request(requests: &Receiver<Request>) -> Option<Request> {
    let mut newest = requests.try_iter().next()?;
    for request in requests.try_iter() {
        newest = request;
    }
    Some(newest)
}
