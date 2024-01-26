//! # User Notification APIs
//!
//! This module provides APIs to send notifications to the desktop environment
//! to display to the user. These notifications are tied to an application but
//! are retained by the system even if the application exits.
//!
//! The underlying mechanism is based on the Freedesktop Notification
//! Specification[^spec].
//!
//! [^spec]: <https://specifications.freedesktop.org/notification-spec/>

// XXX: The current implementation lacks a few advanced features of the
//      Notification Specification. These can be added in the future. It
//      includes things like:
//
//      - Icons: Specifying icones to be displayed next to a notification.
//      - Buttons: Action buttons to allow the user to select a non-default
//                 action.
//      - Actions: Users can interact with notifications and trigger actions
//                 in the raising application. These can even carry metadata
//                 originally supplied by the application.

use crate::application;
use gio;

/// ## Priority Levels
///
/// This type encodes typical priority levels of notifications. See each level
/// for examples when to use them.
#[non_exhaustive]
pub enum Priority {
    /// Low priority, suggested if attention is not required (e.g., weather
    /// reports).
    Low,
    /// Normal priority, suggested as default priority level if attentions is
    /// desired, but not necessarily immediately (e.g., asynchronous messages
    /// like emails, software updates, completed download).
    Normal,
    /// High priority, suggested if attention is desired (e.g., synchronous
    /// messages like chats).
    High,
    /// Urgent priority, suggested if immediate attention is required (e.g.,
    /// phone calls, emergency warnings).
    Urgent,
}

/// ## Notification
///
/// This represents a local notification and its content. Once raised, a handle
/// will track the lifetime of the notification. The notification object can be
/// used to raise multiple, possibly independent notifications with similar
/// content.
pub struct Notification {
    gio: gio::Notification,
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
    id: String,
}

impl From<Priority> for gio::NotificationPriority {
    fn from(v: Priority) -> Self {
        match v {
            Priority::Low => gio::NotificationPriority::Low,
            Priority::Normal => gio::NotificationPriority::Normal,
            Priority::High => gio::NotificationPriority::High,
            Priority::Urgent => gio::NotificationPriority::Urgent,
        }
    }
}

impl Notification {
    /// ## Create Pristine Notification
    ///
    /// Create a pristine notification object ready to be populated with
    /// data.
    pub fn new() -> Self {
        Self::with(None, None, None, None)
    }

    /// ## Create Notification with Content
    ///
    /// Create a new notification object and populate it with the specified
    /// data.
    pub fn with(
        title: Option<&str>,
        message: Option<&str>,
        category: Option<&str>,
        priority: Option<Priority>,
    ) -> Self {
        let mut n = Self {
            gio: gio::Notification::new(title.unwrap_or("")),
        };

        if message.is_some() {
            n.set_message(message);
        }

        if category.is_some() {
            n.set_category(category);
        }

        if let Some(priority_level) = priority {
            n.set_priority(priority_level);
        }

        n
    }

    /// ## Set Title of Notification
    ///
    /// Set the title text of the notification. The content is UTF-8 encoded
    /// and no markup language is supported.
    ///
    /// The specification suggests 40 characters or less for the title. Long
    /// titles will be word-wrapped or truncated.
    pub fn set_title(
        &mut self,
        title: Option<&str>,
    ) -> &mut Self {
        self.gio.set_title(title.unwrap_or(""));
        self
    }

    /// ## Set Notification Message
    ///
    /// Set the message text of the notification. The content is UTF-8 encoded
    /// and basic markup is supported. Each line is formatted as a paragraph.
    /// Content will be word-wrapped if necessary.
    ///
    /// Notification servers might not display notification messages, but
    /// decide to only show the title.
    ///
    /// ### Markup
    ///
    /// Basic markup is supported, but might be stripped by the implementation.
    /// The supported markup uses an XML-like syntax and supports the following
    /// tags:
    ///
    /// - `<b>...</b>`: Bold
    /// - `<i>...</i>`: Italic
    /// - `<u>...</u>`: Underline
    /// - `<a href="...">...</a>`: Hyperlink
    /// - `<img src="..." alt="..."/>`: Image
    ///
    /// Markup is interpreted by the notification server, rather than the
    /// sending client. Hence, implementations might handle markup differently.
     pub fn set_message(
        &mut self,
        message: Option<&str>,
    ) -> &mut Self {
        self.gio.set_body(message);
        self
    }

    /// ## Set Notification Category
    ///
    /// Set the category of the notification. This allows notification servers
    /// to group notifications or display them in a suitable manner.
    ///
    /// Categories are of the form `class.name`, where the class is a broad
    /// category, and the name is a specific sub-category. Common categories
    /// are listed in the Freedesktop Notification Specification [1].
    ///
    /// Custom categories should use `x-vendor.class.name` to avoid
    /// name-clashes.
    ///
    /// [1]: https://specifications.freedesktop.org/notification-spec/
    pub fn set_category(
        &mut self,
        category: Option<&str>,
    ) -> &mut Self {
        self.gio.set_category(category);
        self
    }

    /// ## Set Notification Priority
    ///
    /// Set the priority level of the notification. If not set, `Normal` is
    /// assumed.
    pub fn set_priority(
        &mut self,
        priority: Priority,
    ) -> &mut Self {
        self.gio.set_priority(priority.into());
        self
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
        app: &'ctx application::Context,
        id: Option<String>,
    ) -> Option<Handle<'ctx>> {
        <_ as gio::prelude::ApplicationExt>::send_notification(
            &app.gio,
            id.as_deref(),
            &self.gio,
        );

        if let Some(id_str) = id {
            Some(Handle::with_id(app, id_str))
        } else {
            None
        }
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
        id: String,
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
    pub fn rescind(self) {
        <_ as gio::prelude::ApplicationExt>::withdraw_notification(
            &self.app.gio,
            &self.id,
        );
    }
}
