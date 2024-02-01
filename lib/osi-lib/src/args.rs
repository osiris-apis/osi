//! # Program Arguments
//!
//! XXX

use crate::compat;

/// Error definitions for all possible errors of the argument parser.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error<'args, Id> {
    /// Specified flag contains invalid Unicode.
    FlagInvalidUnicode(&'args compat::OsStr),
    /// Specified flag is not known.
    FlagUnknown(&'args str),
    /// Specified toggle-flag is not known either way.
    FlagToggleUnknown(&'args str),
    /// Specified flag cannot be toggled.
    FlagCannotBeToggled(&'args str),
    /// Specified flag cannot take values.
    FlagTakesNoValue(&'args str, &'args compat::OsStr),
    /// Specified toggle-flag cannot take values.
    FlagToggleTakesNoValue(&'args str, &'args compat::OsStr),
    /// Specified flag needs a value.
    FlagNeedsValue(&'args str),
    /// Value parser for set-flag failed.
    FlagSetValue(&'args str, &'args str),
    /// Value parser for toggle-flag failed.
    FlagToggleValue(&'args str, bool, &'args str),
    /// Value parser for parse-flag failed.
    FlagParseValue(&'args str, &'args compat::OsStr, &'args str),
    /// Short flags are unknown.
    ShortsUnknown(&'args compat::OsStr),
    /// Parameter parser for command failed.
    CommandParameter(Id, &'args compat::OsStr, &'args str),
    /// Specified command takes no parameters.
    CommandTakesNoParameters(Id, &'args compat::OsStr),
}

type Sink<'args, Source> = &'args mut dyn sink::Sink<
    Source,
    Success = (),
    Error = &'args str,
>;

/// Location and parsing information for command-line parameters. This
/// defines how commands take values, and how they are processed when present.
pub type Parameters<'args> = Sink<'args, &'args compat::OsStr>;

/// Location and parsing information for command-line flags. This defines
/// whether a flag takes a value, and how a flag is processed when present.
#[derive(Debug)]
#[non_exhaustive]
pub enum Value<'args> {
    Set(Sink<'args, ()>),
    Toggle(Sink<'args, bool>),
    Parse(Sink<'args, &'args compat::OsStr>),
}

/// Definition of a command-line flag. This carries all information required
/// to parse a specific flag on the command-line and store parsed information.
#[derive(Debug)]
pub struct Flag<'args, 'ctx> {
    name: &'ctx str,
    value: core::cell::RefCell<Value<'args>>,
}

/// Definition of a command-line sub-command. This carries all information
/// required to parse a specific sub-command on the command-line, as well as
/// store parsed information.
#[derive(Debug)]
pub struct Command<'args, 'ctx, Id> {
    id: Id,
    name: &'ctx str,
    commands: &'ctx [Command<'args, 'ctx, Id>],
    flags: &'ctx [Flag<'args, 'ctx>],
    parameters: core::cell::RefCell<Option<Parameters<'args>>>,
}

/// Command-line parser setup, which encapsulates operational flags as well as
/// possible caches for repeated parser operations.
#[derive(Debug)]
pub struct Parser {
}

impl<'args, 'ctx> Flag<'args, 'ctx> {
    fn with(
        name: &'ctx str,
        value: Value<'args>,
    ) -> Self {
        Self {
            name: name,
            value: core::cell::RefCell::new(value),
        }
    }

    /// Create a command-line flag definition with the specified name and value
    /// location. All other properties of the flag will assume their defaults.
    pub fn with_name(
        name: &'ctx str,
        value: Value<'args>,
    ) -> Self {
        Self::with(name, value)
    }
}

impl<'args, 'ctx, Id> Command<'args, 'ctx, Id> {
    fn with(
        id: Id,
        name: &'ctx str,
        commands: &'ctx mut [Command<'args, 'ctx, Id>],
        flags: &'ctx mut [Flag<'args, 'ctx>],
        parameters: Option<Parameters<'args>>,
    ) -> Self {
        commands.sort_unstable_by_key(|v| v.name);
        flags.sort_unstable_by_key(|v| v.name);

        Self {
            id: id,
            name: name,
            commands: commands,
            flags: flags,
            parameters: core::cell::RefCell::new(parameters),
        }
    }

    /// Create an anonymous command definition with the specified sub-commands,
    /// flags, and parameter parser. All other properties of the command will
    /// assume their defaults.
    pub fn new(
        id: Id,
        commands: &'ctx mut [Command<'args, 'ctx, Id>],
        flags: &'ctx mut [Flag<'args, 'ctx>],
        parameters: Option<Parameters<'args>>,
    ) -> Self {
        Self::with(id, "--invalid--", commands, flags, parameters)
    }

    /// Create a command-line command definition with the specified name,
    /// sub-commands, flags, and parameter parser. All other properties of the
    /// command will assume their defaults.
    pub fn with_name(
        id: Id,
        name: &'ctx str,
        commands: &'ctx mut [Command<'args, 'ctx, Id>],
        flags: &'ctx mut [Flag<'args, 'ctx>],
        parameters: Option<Parameters<'args>>,
    ) -> Self {
        Self::with(id, name, commands, flags, parameters)
    }

    fn find_command(
        &self,
        name: &str,
    ) -> Option<&'ctx Command<'args, 'ctx, Id>> {
        match self.commands.binary_search_by_key(
            &name,
            |v| v.name,
        ) {
            Ok(v) => Some(&self.commands[v]),
            _ => None,
        }
    }

    fn find_flag(
        &self,
        name: &str,
    ) -> Option<&'ctx Flag<'args, 'ctx>> {
        match self.flags.binary_search_by_key(
            &name,
            |v| v.name,
        ) {
            Ok(v) => Some(&self.flags[v]),
            _ => None,
        }
    }
}

impl Parser {
    /// Create a new command-line parser with the default settings. This parser
    /// can be used to parse multiple command-lines, if desired.
    pub fn new() -> Self {
        Self {
        }
    }

    fn lookup_flag<'args, 'ctx, Id>(
        history: &alloc::vec::Vec<&'ctx Command<'args, 'ctx, Id>>,
        flag: &str,
    ) -> Option<&'ctx Flag<'args, 'ctx>> {
        for cmd in history.iter().rev() {
            if let Some(v) = cmd.find_flag(flag) {
                return Some(v);
            }
        }

        None
    }

    fn parse_flag<'args, 'ctx, Id, Source>(
        &mut self,
        arguments: &mut Source,
        history: &alloc::vec::Vec<&'ctx Command<'args, 'ctx, Id>>,
        flag_str: &'args str,
        value_opt: Option<&'args compat::OsStr>,
    ) -> Result<(), Error<'args, Id>>
    where
        Source: Iterator<Item = &'args compat::OsStr>,
    {
        let (flag, flag_toggled) = match Self::lookup_flag(history, flag_str) {
            Some(v) => (v, None),
            None => match flag_str.strip_prefix("no-") {
                None => return Err(Error::FlagUnknown(flag_str)),
                Some(stripped) => match Self::lookup_flag(history, stripped) {
                    None => return Err(Error::FlagToggleUnknown(stripped)),
                    Some(v) => (v, Some(stripped)),
                }
            },
        };
        let mut value = flag.value.borrow_mut();

        match (&mut *value, flag_toggled, value_opt) {
            (Value::Set(_), Some(v), _)
            | (Value::Parse(_), Some(v), _) => {
                // Flag only exists without `no-*` prefix, but this flag cannot
                // be toggled. Hence, signal an error and ignore the argument.
                Err(Error::FlagCannotBeToggled(v))
            },
            (Value::Set(_), None, Some(v)) => {
                // Flag is nullary but a value was assigned inline. Signal an
                // error and ignore the argument.
                Err(Error::FlagTakesNoValue(flag_str, v))
            },
            (Value::Toggle(_), t, Some(v)) => {
                // Flag is nullary but a value was assigned inline. Signal an
                // error and ignore the argument.
                Err(Error::FlagToggleTakesNoValue(t.unwrap_or(flag_str), v))
            },
            (Value::Set(s), None, None) => {
                // Correct use of settable-flag.
                s.push(()).map_err(
                    |e| Error::FlagSetValue(flag_str, e),
                )
            },
            (Value::Toggle(s), t, None) => {
                // Correct use of toggle-flag.
                s.push(t.is_none()).map_err(
                    |e| Error::FlagToggleValue(t.unwrap_or(flag_str), t.is_none(), e),
                )
            },
            (Value::Parse(s), None, None) => {
                // Flag requires a value, so fetch it.
                match arguments.next() {
                    None => Err(Error::FlagNeedsValue(flag_str)),
                    Some(v) => s.push(v).map_err(
                        |e| Error::FlagParseValue(flag_str, v, e),
                    )
                }
            },
            (Value::Parse(s), None, Some(v)) => {
                // Flag requires a value that was passed inline.
                s.push(v).map_err(
                    |e| Error::FlagParseValue(flag_str, v, e),
                )
            },
        }
    }

    fn parse_short<'args, 'ctx, Id>(
        &mut self,
        _history: &alloc::vec::Vec<&'ctx Command<'args, 'ctx, Id>>,
        short_str: &'args compat::OsStr,
    ) -> Result<(), Error<'args, Id>> {
        // Our configuration does not allow specifying short options, so none
        // of these can ever match. Hence, treat them all as invalid for now
        // and signal an error. Then ignore the argument and continue.
        Err(Error::ShortsUnknown(short_str))
    }

    fn parse_command<'args, 'ctx, Id>(
        &mut self,
        command: &'ctx Command<'args, 'ctx, Id>,
        arg_os: &'args compat::OsStr,
        arg_str_opt: Option<&'args str>,
    ) -> Result<Option<&'ctx Command<'args, 'ctx, Id>>, Error<'args, Id>>
    where
        Id: Clone,
    {
        let sub_opt = match arg_str_opt {
            None => None,
            Some(arg_str) => command.find_command(arg_str),
        };

        if let Some(sub) = sub_opt {
            Ok(Some(sub))
        } else if let Some(ref mut v) = *command.parameters.borrow_mut() {
            v.push(arg_os).map_err(
                |e| Error::CommandParameter(command.id.clone(), arg_os, e),
            )?;
            Ok(None)
        } else {
            Err(Error::CommandTakesNoParameters(command.id.clone(), arg_os))
        }
    }

    fn parse_root<'args, 'ctx, Id, Source>(
        &mut self,
        mut arguments: Source,
        command: &'ctx Command<'args, 'ctx, Id>,
    ) -> Result<Id, alloc::boxed::Box<[Error<'args, Id>]>>
    where
        Id: Clone,
        Source: Iterator<Item = &'args compat::OsStr>,
    {
        let mut errors = alloc::vec::Vec::new();
        let mut history = alloc::vec![command];
        let mut current = command;

        loop {
            let arg_os = match arguments.next() {
                None => break,
                Some(v) => v,
            };

            // Get the UTF-8 prefix of the argument. Anything we can parse must
            // be valid UTF-8, but some of it might be trailed by arbitrary OS
            // data (e.g., `--path=./some/path` can contain trailing non-UTF-8
            // data). This performs a UTF-8 check on all arguments, but avoids
            // any allocation. Hence, you can parse large data chunks as
            // arguments without incurring anything more expensive than a UTF-8
            // check. For anything bigger than this, you likely want side
            // channels, anyway.
            let arg_bytes = arg_os.as_encoded_bytes();
            let (arg_front, arg_tail) = match core::str::from_utf8(arg_bytes) {
                Ok(v) => (v, false),
                Err(e) => unsafe {
                    // SAFETY: `Utf8Error::valid_up_to()` points exactly at the
                    //         first byte past a valid UTF-8 section, so we can
                    //         safely cast it to a `str` unchecked.
                    let v = &arg_bytes[..e.valid_up_to()];
                    (core::str::from_utf8_unchecked(v), true)
                },
            };

            if let Some(arg_front_dd) = arg_front.strip_prefix("--") {
                // This argument starts with `--` and thus specifies a flag.
                // This can be one of: `--`, `--flag`, `--flag=value`. So first
                // decode the argument into flag and value, then handle the
                // distinct cases.
                let (flag, unknown, value) = match arg_front_dd.split_once('=') {
                    None => (arg_front_dd, arg_tail, None),
                    Some((before, _)) => {
                        let v = unsafe {
                            // SAFETY: We split off a well-defined UTF-8
                            //         sequence, which is allowed for
                            //         `std::ffi::OsStr`.
                            compat::OsStr::from_encoded_bytes_unchecked(
                                &arg_bytes[2+before.len()+1..],
                            )
                        };
                        (before, false, Some(v))
                    },
                };

                match (flag, unknown, value) {
                    (_, true, _) => {
                        // We have invalid UTF-8 as part of the flag name
                        // (i.e., before any possible `=`). This cannot match
                        // any flag we know, so signal an error and ignore it.
                        errors.push(Error::FlagInvalidUnicode(arg_os));
                    },

                    ("", false, None) => {
                        // We got an empty flag. This ends all parsing and
                        // forwards the remaining arguments as parameters.
                        if let Some(ref mut p) = *current.parameters.borrow_mut() {
                            while let Some(v) = arguments.next() {
                                if let Err(e) = p.push(v) {
                                    errors.push(Error::CommandParameter(
                                        current.id.clone(), v, e,
                                    ));
                                }
                            }
                        } else if let Some(v) = arguments.next() {
                            errors.push(Error::CommandTakesNoParameters(
                                current.id.clone(), v,
                            ));
                        }
                    },

                    (_, false, _) => {
                        // We got a complete flag with or without value. Look
                        // up the flag and pass the value along, if required.
                        if let Err(e) = self.parse_flag(&mut arguments, &history, flag, value) {
                            errors.push(e);
                        }
                    },
                }
            } else if arg_bytes.len() >= 2 && arg_bytes[0] == b'-' {
                // A list of short flags was given. Multiple ones might be
                // combined into a single argument. Note that a single dash
                // without following flags has no special meaning and we avoid
                // handling it here.
                if let Err(e) = self.parse_short(&history, arg_os) {
                    errors.push(e);
                }
            } else {
                // This argument is either a sub-command or a parameter of the
                // current command. Sub-commands take preference, everything
                // else is treated as command parameter.
                match self.parse_command(
                    &current,
                    arg_os,
                    (!arg_tail).then_some(arg_front),
                ) {
                    Ok(None) => {},
                    Ok(Some(next)) => {
                        current = next;
                        history.push(current);
                    },
                    Err(e) => errors.push(e),
                }
            }
        }

        if errors.is_empty() {
            Ok(current.id.clone())
        } else {
            Err(errors.into_boxed_slice())
        }
    }

    /// Parse all arguments as command-line arguments for the specified command
    /// definition. On success, return the identifier of the command or
    /// sub-command that was specified on the command-line. All command-line
    /// flags are handled via the specified value handlers of the command
    /// definition.
    ///
    /// ## Errors
    ///
    /// The parser continues operation when encountering a parsing error. All
    /// errors will be collected and then returned to the caller. This allows
    /// producing combined diagnostics for multiple errors, if desired.
    pub fn parse<'args, 'ctx, Id, Source>(
        &mut self,
        arguments: Source,
        command: &'ctx Command<'args, 'ctx, Id>,
    ) -> Result<Id, alloc::boxed::Box<[Error<'args, Id>]>>
    where
        Id: Clone,
        Source: Iterator<Item = &'args compat::OsStr>,
    {
        self.parse_root(arguments, command)
    }

    /// Parse all arguments as command-line arguments.
    ///
    /// This variant requires the arguments to be valid Rust strings. See
    /// `Self::parse()` for details on the operation.
    pub fn parse_str<'args, 'ctx, Id, Source, SourceItem>(
        &mut self,
        arguments: Source,
        command: &'ctx Command<'args, 'ctx, Id>,
    ) -> Result<Id, alloc::boxed::Box<[Error<'args, Id>]>>
    where
        Id: Clone,
        Source: IntoIterator<Item = &'args SourceItem>,
        SourceItem: AsRef<str> + 'args,
    {
        self.parse(
            arguments.into_iter().map(|v| v.as_ref().into()),
            command,
        )
    }

    /// Parse all arguments as command-line arguments.
    ///
    /// This variant requires the arguments to be valid Rust `compat::OsStr`. See
    /// `Self::parse()` for details on the operation.
    #[cfg(feature = "std")]
    pub fn parse_osstr<'args, 'ctx, Id, Source, SourceItem>(
        &mut self,
        arguments: Source,
        command: &'ctx Command<'args, 'ctx, Id>,
    ) -> Result<Id, alloc::boxed::Box<[Error<'args, Id>]>>
    where
        Id: Clone,
        Source: IntoIterator<Item = &'args SourceItem>,
        SourceItem: AsRef<std::ffi::OsStr> + 'args,
    {
        self.parse(
            arguments.into_iter().map(|v| compat::OsStr::from_osstr(v.as_ref())),
            command,
        )
    }
}

pub mod sink {
    //! # Interfaces for Generic Data Sinks
    //!
    //! Data sinks allow generalizing the way how data is collected or stored.
    //! The `Sink` trait defines how any type can accept specific input data
    //! and store it, possibly raising errors if the data could not be parsed.

    use crate::compat;

    /// Generic data sinks define how data is collected and stored, providing a
    /// uniform interface to the caller. Sinks are specific to the type of the
    /// source data, and can be implemented for a wide range of different
    /// sources.
    pub trait Sink<Source>
    where
        Self: core::fmt::Debug,
    {
        /// Data type used when data was successfully stored.
        type Success;
        /// Data type used when data could not be stored.
        type Error;

        /// Push data into the sink, reporting whether it was stored
        /// successfully. Usually, this requires the implementor to parse the
        /// input data (if necessary) and then store it.
        ///
        /// It is up to the implementor to decide whether new data overrides
        /// old data, or whether it is amended.
        fn push(
            &mut self,
            data: Source,
        ) -> Result<Self::Success, Self::Error>;
    }

    impl<'args> Sink<&'args compat::OsStr> for &'args compat::OsStr {
        type Success = ();
        type Error = &'args str;

        fn push(
            &mut self,
            data: &'args compat::OsStr,
        ) -> Result<Self::Success, Self::Error> {
            *self = data;
            Ok(())
        }
    }

    #[cfg(feature = "std")]
    impl<'args> Sink<&'args compat::OsStr> for &'args std::ffi::OsStr {
        type Success = ();
        type Error = &'args str;

        fn push(
            &mut self,
            data: &'args compat::OsStr,
        ) -> Result<Self::Success, Self::Error> {
            *self = data.as_osstr();
            Ok(())
        }
    }

    #[cfg(feature = "std")]
    impl<'args> Sink<&'args compat::OsStr> for std::ffi::OsString {
        type Success = ();
        type Error = &'args str;

        fn push(
            &mut self,
            data: &'args compat::OsStr,
        ) -> Result<Self::Success, Self::Error> {
            *self = data.as_osstr().into();
            Ok(())
        }
    }

    impl<'args> Sink<&'args compat::OsStr> for &'args str {
        type Success = ();
        type Error = &'args str;

        fn push(
            &mut self,
            data: &'args compat::OsStr,
        ) -> Result<Self::Success, Self::Error> {
            if let Ok(data_str) = data.to_str() {
                *self = data_str;
                Ok(())
            } else {
                Err("Does not accept non-Unicode values")
            }
        }
    }

    impl<'args> Sink<&'args compat::OsStr> for alloc::string::String {
        type Success = ();
        type Error = &'args str;

        fn push(
            &mut self,
            data: &'args compat::OsStr,
        ) -> Result<Self::Success, Self::Error> {
            if let Ok(data_str) = data.to_str() {
                *self = data_str.into();
                Ok(())
            } else {
                Err("Does not accept non-Unicode values")
            }
        }
    }

    impl<'args> Sink<&'args compat::OsStr> for bool {
        type Success = ();
        type Error = &'args str;

        fn push(
            &mut self,
            data: &'args compat::OsStr,
        ) -> Result<Self::Success, Self::Error> {
            if let Ok(data_str) = data.to_str() {
                match data_str {
                    "TRUE" | "True" | "true"
                        | "YES" | "Yes" | "yes"
                        | "ON" | "On" | "on" => {
                        *self = true;
                        Ok(())
                    },
                    "FALSE" | "False" | "false"
                        | "NO" | "No" | "no"
                        | "OFF" | "Off" | "off" => {
                        *self = false;
                        Ok(())
                    },
                    _ => {
                        Err("Does not accept non-boolean values")
                    },
                }
            } else {
                Err("Does not accept non-Unicode values")
            }
        }
    }

    impl<Source, Target> Sink<Source> for Option<Target>
    where
        Target: Sink<Source> + Default,
    {
        type Success = Target::Success;
        type Error = Target::Error;

        fn push(
            &mut self,
            data: Source,
        ) -> Result<Self::Success, Self::Error> {
            self.get_or_insert_with(Default::default)
                .push(data)
        }
    }

    impl<Source, Target> Sink<Source> for alloc::vec::Vec<Target>
    where
        Target: Sink<Source> + Default,
    {
        type Success = Target::Success;
        type Error = Target::Error;

        fn push(
            &mut self,
            data: Source,
        ) -> Result<Self::Success, Self::Error> {
            let mut v: Target = Default::default();
            let r = v.push(data)?;
            self.push(v);
            Ok(r)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::rust_2021::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Id {
        Root,
        Foo,
        Bar,
    }

    #[derive(Debug, Default, Eq, PartialEq)]
    struct Values {
        foo: Option<String>,
        bar: Option<String>,
        foofoo: Option<String>,
        foobar: Option<String>,
        barfoo: Option<String>,
        barbar: Option<String>,
    }

    fn parse<'args>(
        arguments: &'args [&'args str],
        values: &'args mut Values,
    ) -> Result<Id, alloc::boxed::Box<[Error<'args, Id>]>> {
        let mut flags_foo = [
            Flag::with_name("foofoo", Value::Parse(&mut values.foofoo)),
            Flag::with_name("foobar", Value::Parse(&mut values.foobar)),
        ];
        let mut flags_bar = [
            Flag::with_name("barfoo", Value::Parse(&mut values.barfoo)),
            Flag::with_name("barbar", Value::Parse(&mut values.barbar)),
        ];
        let mut cmds = [
            Command::with_name(Id::Foo, "foo", &mut [], &mut flags_foo, None),
            Command::with_name(Id::Bar, "bar", &mut [], &mut flags_bar, None),
        ];
        let mut flags = [
            Flag::with_name("foo", Value::Parse(&mut values.foo)),
            Flag::with_name("bar", Value::Parse(&mut values.bar)),
        ];
        let cmd = Command::new(Id::Root, &mut cmds, &mut flags, None);
        Parser::new().parse_str(arguments, &cmd)
    }

    #[test]
    fn test_basic() {
        let mut values: Values = Default::default();

        let r = parse(
            &["foo", "--foo", "value-foo"],
            &mut values,
        ).unwrap();
        assert_eq!(r, Id::Foo);
        assert_eq!(
            values,
            Values {
                foo: Some("value-foo".into()),
                ..Default::default()
            },
        );

        let r = parse(
            &["--foo", "value-foo", "bar", "--bar", "value-bar", "--barbar", "value-barbar"],
            &mut values,
        ).unwrap();
        assert_eq!(r, Id::Bar);
        assert_eq!(
            values,
            Values {
                foo: Some("value-foo".into()),
                bar: Some("value-bar".into()),
                barbar: Some("value-barbar".into()),
                ..Default::default()
            },
        );
    }

    #[test]
    fn test_errors() {
        let mut values: Values = Default::default();

        let r = parse(
            &["invalid"],
            &mut values,
        ).unwrap_err();
        assert_eq!(r.len(), 1);
        assert!(core::matches!(
            r[0],
            Error::CommandTakesNoParameters(Id::Root, _),
        ));
    }
}
