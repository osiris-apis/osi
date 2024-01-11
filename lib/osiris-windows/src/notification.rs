//! # User Notification APIs
//!
//! This module provides APIs to send notifications to the desktop environment
//! to display to the user. These notifications are tied to an application but
//! are retained by the system even if the application exits.
//!
//! The underlying mechanism is based on the WinRT Toast API [^toast].
//!
//! [^toast]: https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/toast-notifications-overview

// XXX: The current implementation lacks a few advanced features of the
//      WinRT Toast API. These can be added in the future. It includes things
//      like:
//
//      - Icons: Specifying icones to be displayed next to a notification.
//      - Buttons: Action buttons to allow the user to select a non-default
//                 action.

use crate::application;
use windows;

/// ## Notification
///
/// This represents a local notification and its content. Once raised, a handle
/// will track the lifetime of the notification. The notification object can be
/// used to raise multiple, possibly independent notifications with similar
/// content.
pub struct Notification {
}

/// ## Notification Handle
///
/// A live notification can be tracked with a handle. This allows interacting
/// with the notification, once it was raised.
///
/// Dropping the handle has no effect. Raised notifications will be kept even
/// if the application exits. However, the handle allows rescinding raised
/// notifications, if desired.
///
/// A notification handle is tied to an application context and cannot outlive
/// it. However, a notification handle can be recreated from an application
/// context and a notification ID.
pub struct Handle {
}

impl Notification {
    /// ## Create Pristine Notification
    ///
    /// Create a pristine notification object ready to be populated with
    /// data.
    pub fn new() -> Self {
        Self {
        }
    }

    /// ## Raise Notification
    ///
    /// Raise the notification and send them to the notification server. This
    /// will use the specified application context to access the required
    /// system APIs.
    ///
    /// If a notification identifier is specified, any previous notification
    /// of the same identifier is replaced and a handle for the notification
    /// is returned. This handle can be used to rescind the notification.
    ///
    /// If no notification identifier is specified, the notification is still
    /// raised, but no handle is returned, nor is any other notification
    /// replaced.
    pub fn raise<'ctx>(
        &self,
        app: &'ctx application::Application,
        id: Option<String>,
    ) -> Option<()> {
        let xml = windows::Data::Xml::Dom::XmlDocument::new().unwrap();

        xml.LoadXml(
            &windows::core::HSTRING::from(
                r#"
                <toast
                    duration="long"
                    scenario="reminder"
                >
                    <visual>
                        <binding
                            template="ToastGeneric"
                        >
                            <text id="1">Title</text>
                        </binding>
                    </visual>
                </toast>"#,
            ),
        ).unwrap();

        let notification = windows::UI::Notifications::ToastNotification::CreateToastNotification(
            &xml,
        ).unwrap();

        let toast_notifier = windows::UI::Notifications::ToastNotificationManager::CreateToastNotifierWithId(
            &windows::core::HSTRING::from(&app.id),
        ).unwrap();

        toast_notifier.Show(&notification).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        if let Some(_id_str) = id {
            Some(())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show() {
        let app = application::Application::new();
        let notify = Notification::new();

        notify.raise(&app, None);
    }
}
