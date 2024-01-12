//! # User Notification APIs
//!
//! This module provides APIs to send notifications to the Windows Desktop
//! to display to the user. These notifications are tied to an application but
//! are retained by the system even if the application exits.
//!
//! The underlying mechanism is based on the WinRT Toast API [^toast].
//!
//! [^toast]: https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/toast-notifications-overview

// XXX: The current implementation lacks a few advanced features of the
//      WinRT Toast API. These can be added in the future. It includes things
//      like buttons and input controls.

// XXX: Text-content starting with `ms-resource:*` references content in the
//      application resources. We must carefully sanitize user-input to
//      prevent accidental mis-use.
//      This is a horrible layer violation by the WinRT API, yet little we
//      can do about it.

use crate::application;
use windows;

/// ## Notification Duration
///
/// Notifications are shown on screen for a moment. Once they vanish, they are
/// retained in the notification center. This type allows controlling how long
/// it stays visible on screen.
pub enum Duration {
    Short,
    Long,
}

/// ## Notification Scenario
///
/// The scenario type controls how a notification is handled by the
/// notification system, how urgent it is, which default sounds and behaviors
/// to apply, and how to render it.
pub enum Scenario {
    Alarm,
    IncomingCall,
    Reminder,
    Urgent,
}

impl Default for Scenario {
    fn default() -> Self {
        Scenario::Reminder
    }
}

/// ## Notification
///
/// This represents a local notification and its content. Once raised, a handle
/// will track the lifetime of the notification. The notification object can be
/// used to raise multiple, possibly independent notifications with similar
/// content.
#[derive(Default)]
pub struct Notification {
    /// Additional text element used to denote the origin of the notification.
    pub attribution: Option<String>,
    /// Time to display, formatted using the ISO 8601 standard.
    pub display_timestamp: Option<String>,
    /// Duration to keep the notification on screen for.
    pub duration: Option<Duration>,
    /// Header information to group notifications.
    pub header: Option<(String, String)>,
    /// Scenario information for the notification.
    pub scenario: Scenario,
    /// Text elements of the notification.
    pub text: Vec<String>,
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
pub struct Handle<'ctx> {
    app: &'ctx application::Context,
    id: (Option<String>, Option<String>),
}

impl Notification {
    /// ## Create Pristine Notification
    ///
    /// Create a pristine notification object ready to be populated with
    /// data.
    pub fn new() -> Self {
        Default::default()
    }

    fn build_xml(
        &self,
    ) -> Result<
        windows::Data::Xml::Dom::XmlDocument,
        Box<dyn std::error::Error>
    > {
        // Build the Notification XML
        //
        // The content of a notification (and some metadata) is all encoded in
        // an XML document specified by Microsoft [^xml]. It should be rather
        // straightforward to follow.
        //
        // [^xml]: https://learn.microsoft.com/en-us/uwp/schemas/tiles/toastschema/root-elements
        let xml = windows::Data::Xml::Dom::XmlDocument::new()?;

        // Create top-level <toast>
        //
        // This is the root element and defines basic notification properties
        // like duration and scenario.
        let v_toast = xml.CreateElement(windows::core::h!("toast"))?;
        if let Some(ref v) = self.duration {
            v_toast.SetAttribute(
                windows::core::h!("duration"),
                &match v {
                    Duration::Short => windows::core::HSTRING::from("short"),
                    Duration::Long => windows::core::HSTRING::from("long"),
                },
            )?;
        }
        if let Some(ref v) = self.display_timestamp {
            v_toast.SetAttribute(
                windows::core::h!("displayTimeStamp"),
                &windows::core::HSTRING::from(v),
            )?;
        }
        v_toast.SetAttribute(
            windows::core::h!("scenario"),
            &match self.scenario {
                Scenario::Alarm => windows::core::HSTRING::from("alarm"),
                Scenario::IncomingCall => windows::core::HSTRING::from("incomingCall"),
                Scenario::Reminder => windows::core::HSTRING::from("reminder"),
                Scenario::Urgent => windows::core::HSTRING::from("urgent"),
            },
        )?;

        // Create <header>
        //
        // A header allows grouping notifications. It has an ID to identify
        // it, as well as a title that is shown by the notification system.
        if let Some((ref id, ref title)) = self.header {
            let v_header = xml.CreateElement(windows::core::h!("header"))?;
            v_header.SetAttribute(
                windows::core::h!("id"),
                &windows::core::HSTRING::from(id),
            )?;
            v_header.SetAttribute(
                windows::core::h!("title"),
                &windows::core::HSTRING::from(title),
            )?;
            v_toast.AppendChild(&v_header)?;
        }

        // Create <visual>
        //
        // This contains all visual elements that are not auto-generated. That
        // is, this contains text and image elements to show in the
        // notification.
        let v_visual = xml.CreateElement(windows::core::h!("visual"))?;

        // Create <binding>
        //
        // This element used to bind to a particular template, yet nowadays is
        // used to build up the generic template. It is effectively a wrapper
        // around the visual content.
        let v_binding = xml.CreateElement(windows::core::h!("binding"))?;
        v_binding.SetAttribute(
            windows::core::h!("template"),
            windows::core::h!("ToastGeneric"),
        )?;

        // Append <text> Nodes
        //
        // Up to 3 text-nodes are supported, each being rendered as a an
        // individual line. An additional text-node can be used to render the
        // source information of the notification (called `attribution`).
        //
        // We allow adding any number of text-nodes, but these will likely be
        // ignored by the notification server.
        let mut n_text = 0;
        for v in &self.text {
            let v_text = xml.CreateElement(windows::core::h!("text"))?;
            n_text += 1;
            v_text.SetAttribute(
                windows::core::h!("id"),
                &windows::core::HSTRING::from(format!("{}", n_text)),
            )?;
            v_text.SetInnerText(&windows::core::HSTRING::from(v))?;
            v_binding.AppendChild(&v_text)?;
        }
        if let Some(ref v) = self.attribution {
            let v_text = xml.CreateElement(windows::core::h!("text"))?;
            n_text += 1;
            v_text.SetAttribute(
                windows::core::h!("id"),
                &windows::core::HSTRING::from(format!("{}", n_text)),
            )?;
            v_text.SetInnerText(&windows::core::HSTRING::from(v))?;
            v_binding.AppendChild(&v_text)?;
        }

        v_visual.AppendChild(&v_binding)?;
        v_toast.AppendChild(&v_visual)?;
        xml.AppendChild(&v_toast)?;

        Ok(xml)
    }

    /// ## Raise Notification
    ///
    /// Raise the notification and send them to the notification server. This
    /// will operate on behalf of the running process and its application ID.
    /// A suitable application context must be provided.
    ///
    /// If a notification identifier is specified, any previous notification
    /// of the same identifier is replaced and a handle for the notification
    /// is returned. This handle can be used to rescind the notification.
    ///
    /// If no notification identifier is specified, the notification is still
    /// raised, but no handle is returned, nor is any other notification
    /// replaced.
    ///
    /// The notification identifier is a pair of group and tag. Both must match
    /// for the identifier to be considered identical. You are free to use only
    /// one of them. They used to be limited to 16 characters, which was later
    /// extended to 64 characters.
    pub fn raise<'ctx>(
        &self,
        app: &'ctx application::Context,
        id: (Option<String>, Option<String>),
    ) -> Result<Handle<'ctx>, Box<dyn std::error::Error>> {
        // `GetDefault()` queries the current user and `CreateToastNotifier()`
        // queries the AUMID of the application. Hence, claim the application
        // context to have something where we could reasonbly tie that
        // information to.
        app.claim();

        let mgr = windows::UI::Notifications::ToastNotificationManager::GetDefault()?;
        let notifier = mgr.CreateToastNotifier()?;
        let xml = self.build_xml()?;

        let notification = windows::UI::Notifications::ToastNotification::CreateToastNotification(&xml)?;
        if let Some(ref v) = id.0 {
            notification.SetGroup(&windows::core::HSTRING::from(v))?;
        }
        if let Some(ref v) = id.1 {
            notification.SetTag(&windows::core::HSTRING::from(v))?;
        }

        notifier.Show(&notification)?;

        Ok(Handle::with_id(app, id))
    }
}

impl<'ctx> Handle<'ctx> {
    /// ## Create Notification Handle from its ID
    ///
    /// Re-create a notification handle from its ID. This handle will be
    /// functionally identical to the original handle. The application
    /// context does not have to be the same as was used to raise the
    /// notification. However, it must have a matching application ID set.
    pub fn with_id(
        app: &'ctx application::Context,
        id: (Option<String>, Option<String>),
    ) -> Self {
        Self {
            app: app,
            id: id,
        }
    }

    /// ## Rescind a Notification
    ///
    /// Rescind a notification that was previously raised. This will remove
    /// the notification from the notification server and consume the handle.
    pub fn rescind(
        self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // If no ID was used, it was a one-shot notification that cannot be
        // rescinded (or is automatically rescinded). Nothing to do.
        if self.id.0.is_none() && self.id.1.is_none() {
            return Ok(());
        }

        // `GetDefault()` queries the current user and `CreateToastNotifier()`
        // queries the AUMID of the application. Hence, claim the application
        // context to have something where we could reasonbly tie that
        // information to.
        self.app.claim();

        let mgr = windows::UI::Notifications::ToastNotificationManager::GetDefault()?;
        let notifier = mgr.CreateToastNotifier()?;

        // Create a simple template notification to avoid complex and
        // unnecessary XML operations.
        let xml = windows::UI::Notifications::ToastNotificationManager::GetTemplateContent(
            windows::UI::Notifications::ToastTemplateType::ToastText01,
        )?;
        let notification = windows::UI::Notifications::ToastNotification::CreateToastNotification(&xml)?;

        // Set the same IDs as the original notification.
        if let Some(ref v) = self.id.0 {
            notification.SetGroup(&windows::core::HSTRING::from(v))?;
        }
        if let Some(ref v) = self.id.1 {
            notification.SetTag(&windows::core::HSTRING::from(v))?;
        }

        // Remove the notification.
        notifier.Hide(&notification)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show() {
        let app = application::Setup {
            aumid: None,
        }.initialize().unwrap();

        let notify = Notification::new();

        notify.raise(&app, (None, None)).unwrap();
    }
}
