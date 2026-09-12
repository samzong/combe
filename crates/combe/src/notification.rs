use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::{self, NonNull};
use std::time::{Duration, Instant};

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObject, NSObjectProtocol};
use objc2::{AnyThread, class, define_class, extern_protocol, msg_send};
use objc2_foundation::{NSArray, NSError, NSOperationQueue, NSString, NSTimer};

use crate::{habits, surface::SurfaceView, window};

#[link(name = "UserNotifications", kind = "framework")]
unsafe extern "C" {
    static UNNotificationDefaultActionIdentifier: &'static NSString;
}

pub struct Notice {
    title: String,
    body: String,
    priority: u8,
}

impl Notice {
    pub fn desktop(title: String, body: String) -> Self {
        let title = clean(&title, 128);
        Self {
            title: if title.is_empty() {
                "Terminal needs attention".into()
            } else {
                title
            },
            body: clean(&body, 2048),
            priority: 2,
        }
    }

    pub fn bell() -> Self {
        Self {
            title: "Terminal needs attention".into(),
            body: String::new(),
            priority: 0,
        }
    }

    pub fn command(exit: i16, nanos: u64) -> Option<Self> {
        let duration = Duration::from_nanos(nanos);
        if duration < habits::COMMAND_NOTIFY_AFTER {
            return None;
        }
        Some(Self {
            title: match exit {
                0 => "Command succeeded",
                1.. => "Command failed",
                _ => "Command finished",
            }
            .into(),
            body: if exit < 0 {
                format!("Finished after {} seconds.", duration.as_secs())
            } else {
                format!(
                    "Finished after {} seconds with exit code {exit}.",
                    duration.as_secs()
                )
            },
            priority: 1,
        })
    }
}

fn clean(text: &str, limit: usize) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(limit)
        .collect::<String>()
        .trim()
        .to_owned()
}

#[derive(Default)]
struct Pending {
    notice: Option<Notice>,
    due: Option<Instant>,
    delivered: Option<Instant>,
}

impl Pending {
    fn receive(&mut self, notice: Notice, now: Instant) {
        if self
            .notice
            .as_ref()
            .is_none_or(|old| notice.priority >= old.priority)
        {
            self.notice = Some(notice);
        }
        self.due.get_or_insert_with(|| {
            (now + habits::NOTIFICATION_BATCH).max(
                self.delivered
                    .map_or(now, |last| last + habits::NOTIFICATION_INTERVAL),
            )
        });
    }

    fn cancel(&mut self) {
        self.notice = None;
        self.due = None;
    }
}

#[derive(Default)]
struct State {
    delegate: Option<Retained<Delegate>>,
    pending: HashMap<String, Pending>,
    timer: Option<(Instant, Retained<NSTimer>)>,
    authorizing: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

extern_protocol!(
    #[allow(clippy::missing_safety_doc)]
    unsafe trait UNUserNotificationCenterDelegate: NSObjectProtocol {}
);

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "CombeNotificationDelegate"]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}

    unsafe impl UNUserNotificationCenterDelegate for Delegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &AnyObject,
            notification: &AnyObject,
            completion: &DynBlock<dyn Fn(usize)>,
        ) {
            let id = notification_id(notification);
            let completion = completion.copy();
            let block = RcBlock::new(move || {
                let present = window::notification_source(&id)
                    .is_some_and(|(view, _)| view.needs_attention() && !window::is_observed(&view));
                completion.call((if present {
                    (1 << 4) | (1 << 3) | (1 << 1)
                } else {
                    0
                },));
            });
            unsafe {
                NSOperationQueue::mainQueue().addOperationWithBlock(&block);
            }
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &AnyObject,
            response: &AnyObject,
            completion: &DynBlock<dyn Fn()>,
        ) {
            let notification: Retained<AnyObject> = unsafe { msg_send![response, notification] };
            let id = notification_id(&notification);
            let action: Retained<NSString> = unsafe { msg_send![response, actionIdentifier] };
            let activate = *action == *unsafe { UNNotificationDefaultActionIdentifier };
            let completion = completion.copy();
            let block = RcBlock::new(move || {
                if activate {
                    window::focus_notification(&id);
                }
                completion.call(());
            });
            unsafe {
                NSOperationQueue::mainQueue().addOperationWithBlock(&block);
            }
        }
    }
);

fn notification_id(notification: &AnyObject) -> String {
    unsafe {
        let request: Retained<AnyObject> = msg_send![notification, request];
        let id: Retained<NSString> = msg_send![&request, identifier];
        id.to_string()
    }
}

fn on_main(work: impl Fn() + Send + 'static) {
    unsafe { NSOperationQueue::mainQueue().addOperationWithBlock(&RcBlock::new(work)) };
}

fn center() -> Retained<AnyObject> {
    unsafe { msg_send![class!(UNUserNotificationCenter), currentNotificationCenter] }
}

pub fn init() {
    let delegate: Retained<Delegate> = unsafe { msg_send![Delegate::alloc(), init] };
    unsafe {
        let _: () = msg_send![&center(), setDelegate: &*delegate];
    }
    STATE.with(|state| state.borrow_mut().delegate = Some(delegate));
}

pub fn receive(view: &SurfaceView, notice: Notice) {
    if window::is_observed(view) {
        return;
    }
    view.set_attention(true);
    STATE.with(|state| {
        state
            .borrow_mut()
            .pending
            .entry(view.notification_id().to_owned())
            .or_default()
            .receive(notice, Instant::now())
    });
    schedule();
}

fn schedule() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let due = if state.authorizing {
            None
        } else {
            state.pending.values().filter_map(|item| item.due).min()
        };
        if state.timer.as_ref().map(|(due, _)| *due) == due {
            return;
        }
        if let Some((_, timer)) = state.timer.take() {
            timer.invalidate();
        }
        let Some(due) = due else {
            return;
        };
        let block = RcBlock::new(|_: NonNull<NSTimer>| request_authorization());
        state.timer = Some((due, unsafe {
            NSTimer::scheduledTimerWithTimeInterval_repeats_block(
                due.saturating_duration_since(Instant::now())
                    .as_secs_f64()
                    .max(0.001),
                false,
                &block,
            )
        }));
    });
}

fn request_authorization() {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.timer = None;
        state.authorizing = true;
    });
    let block = RcBlock::new(|granted: Bool, error: *mut NSError| {
        if let Some(error) = unsafe { error.as_ref() } {
            eprintln!("combe: notification authorization: {error}");
        }
        let granted = granted.as_bool();
        on_main(move || flush(granted));
    });
    unsafe {
        let _: () = msg_send![&center(), requestAuthorizationWithOptions: (1usize << 2) | (1usize << 1), completionHandler: &*block];
    }
}

fn flush(granted: bool) {
    let now = Instant::now();
    let notices = STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.authorizing = false;
        state
            .pending
            .iter_mut()
            .filter_map(|(id, pending)| {
                if pending.due.is_none_or(|due| due > now) {
                    return None;
                }
                pending.due = None;
                let notice = pending.notice.take()?;
                if granted {
                    pending.delivered = Some(now);
                }
                Some((id.clone(), notice))
            })
            .collect::<Vec<_>>()
    });
    if granted {
        for (id, notice) in notices {
            if let Some((view, workspace)) = window::notification_source(&id)
                && view.needs_attention()
                && !window::is_observed(&view)
            {
                deliver(&id, &workspace, notice);
            }
        }
    }
    schedule();
}

fn deliver(id: &str, workspace: &str, notice: Notice) {
    unsafe {
        let content: Retained<AnyObject> = msg_send![class!(UNMutableNotificationContent), new];
        let _: () = msg_send![&content, setTitle: &*NSString::from_str(&notice.title)];
        let _: () = msg_send![&content, setSubtitle: &*NSString::from_str(&clean(workspace, 128))];
        let _: () = msg_send![&content, setBody: &*NSString::from_str(&notice.body)];
        let sound: Retained<AnyObject> = msg_send![class!(UNNotificationSound), defaultSound];
        let _: () = msg_send![&content, setSound: &*sound];
        let request: Retained<AnyObject> = msg_send![class!(UNNotificationRequest), requestWithIdentifier: &*NSString::from_str(id), content: &*content, trigger: ptr::null::<AnyObject>()];
        let id = id.to_owned();
        let block = RcBlock::new(move |error: *mut NSError| {
            if let Some(error) = error.as_ref() {
                eprintln!("combe: notification delivery: {error}");
            }
            let id = id.clone();
            on_main(move || {
                if window::notification_source(&id)
                    .is_none_or(|(view, _)| !view.needs_attention() || window::is_observed(&view))
                {
                    remove_delivered(&id);
                }
            });
        });
        let _: () =
            msg_send![&center(), addNotificationRequest: &*request, withCompletionHandler: &*block];
    }
}

fn remove_delivered(id: &str) {
    let ids = NSArray::from_slice(&[&*NSString::from_str(id)]);
    unsafe {
        let center = center();
        let _: () = msg_send![&center, removePendingNotificationRequestsWithIdentifiers: &*ids];
        let _: () = msg_send![&center, removeDeliveredNotificationsWithIdentifiers: &*ids];
    }
}

pub fn acknowledge(id: &str) {
    STATE.with(|state| {
        if let Some(pending) = state.borrow_mut().pending.get_mut(id) {
            pending.cancel();
        }
    });
    remove_delivered(id);
    schedule();
}

pub fn forget(id: &str) {
    STATE.with(|state| {
        state.borrow_mut().pending.remove(id);
    });
    remove_delivered(id);
    schedule();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bursts_keep_explicit_text_and_independent_panes_without_postponing_delivery() {
        let now = Instant::now();
        let mut a = Pending::default();
        let mut b = Pending::default();
        a.receive(
            Notice::desktop("Build ready".into(), "Complete".into()),
            now,
        );
        a.receive(Notice::bell(), now + Duration::from_millis(100));
        b.receive(Notice::bell(), now + Duration::from_millis(100));
        assert_eq!(a.notice.as_ref().unwrap().title, "Build ready");
        assert_eq!(a.due, Some(now + habits::NOTIFICATION_BATCH));
        a.delivered = Some(now);
        a.cancel();
        a.receive(Notice::bell(), now + Duration::from_millis(200));
        assert_eq!(a.due, Some(now + habits::NOTIFICATION_INTERVAL));
        assert_eq!(
            b.due,
            Some(now + Duration::from_millis(100) + habits::NOTIFICATION_BATCH)
        );
        assert!(Notice::command(0, 1_000_000_000).is_none());
        assert!(Notice::command(7, 6_000_000_000).is_some());
    }
}
