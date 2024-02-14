//! # Program Arguments
//!
//! This module implements a basic command-line parser for runtime arguments
//! passed to a program.

use crate::compat;

/// Error definitions for all possible errors of the argument parser.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error<'args> {
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
    FlagSetValue(&'args str, sink::Error),
    /// Value parser for toggle-flag failed.
    FlagToggleValue(&'args str, bool, sink::Error),
    /// Value parser for parse-flag failed.
    FlagParseValue(&'args str, &'args compat::OsStr, sink::Error),
    /// Short flags are unknown.
    ShortsUnknown(&'args compat::OsStr),
    /// Parameter parser for command failed.
    CommandParameter(alloc::string::String, &'args compat::OsStr, sink::Error),
    /// Specified command takes no parameters.
    CommandTakesNoParameters(alloc::string::String, &'args compat::OsStr),
}

// Type alias for value parsers.
type Sink<'args, Id, Source> = &'args dyn sink::Sink<Id, Source>;

/// Location and parsing information for command-line parameters. This
/// defines how commands take values, and how they are processed when present.
pub type Parameters<'args, Id> = Sink<'args, Id, &'args compat::OsStr>;

/// Location and parsing information for command-line flags. This defines
/// whether a flag takes a value, and how a flag is processed when present.
#[derive(Debug)]
#[non_exhaustive]
pub enum Value<'args, Id> {
    Set(Sink<'args, Id, ()>),
    Toggle(Sink<'args, Id, bool>),
    Parse(Sink<'args, Id, &'args compat::OsStr>),
}

/// An audited list of command-line configuration.
///
/// This type encodes auditing guarantees in the type-system. It takes user
/// configuration, audits it, and then provides a wrapper type to ensure the
/// auditing is encoded in the type-system.
#[derive(Debug)]
#[repr(transparent)]
pub struct AuditedList<T: ?Sized> {
    list: T,
}

/// Definition of a command-line flag. This carries all information required
/// to parse a specific flag on the command-line and store parsed information.
#[derive(Debug)]
pub struct Flag<'args, 'ctx, Id> {
    name: &'ctx str,
    value: Value<'args, Id>,

    help_short: Option<&'ctx str>,
}

/// An audited list of command-line flags.
pub type FlagList<'args, 'ctx, const N: usize, Id> = AuditedList<[Flag<'args, 'ctx, Id>; N]>;

/// A reference to an audited list of command-line flags.
pub type FlagListRef<'args, 'ctx, Id> = &'ctx AuditedList<[Flag<'args, 'ctx, Id>]>;

/// Definition of a command-line sub-command. This carries all information
/// required to parse a specific sub-command on the command-line, as well as
/// store parsed information.
#[derive(Debug)]
pub struct Command<'args, 'ctx, Id> {
    id: Id,
    name: &'ctx str,
    commands: CommandListRef<'args, 'ctx, Id>,
    flags: FlagListRef<'args, 'ctx, Id>,
    parameters: Option<Parameters<'args, Id>>,

    help_short: Option<&'ctx str>,
}

/// An audited list of command-line sub-commands.
pub type CommandList<'args, 'ctx, const N: usize, Id> = AuditedList<[Command<'args, 'ctx, Id>; N]>;

/// A reference to an audited list of command-line sub-commands.
pub type CommandListRef<'args, 'ctx, Id> = &'ctx AuditedList<[Command<'args, 'ctx, Id>]>;

/// Command-line parser setup, which encapsulates operational flags as well as
/// possible caches for repeated parser operations.
#[derive(Debug)]
pub struct Parser {
}

/// Flag collector for standard `--help` flags. It implements `sink::Sink` and
/// always stores the last calling context as value.
#[derive(Debug)]
pub struct Help<Id> {
    cell: core::cell::RefCell<Option<Id>>,
}

impl<'args> core::fmt::Display for Error<'args> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            Self::FlagInvalidUnicode(flag) => fmt.write_fmt(core::format_args!("Flag name contains invalid Unicode: {}", flag.to_string_lossy())),
            Self::FlagUnknown(flag) => fmt.write_fmt(core::format_args!("Invalid flag name: --{}", flag)),
            Self::FlagToggleUnknown(flag) => fmt.write_fmt(core::format_args!("Invalid toggle-flag name: --[no-]{}", flag)),
            Self::FlagCannotBeToggled(flag) => fmt.write_fmt(core::format_args!("Flag cannot be toggled: --no-{}", flag)),
            Self::FlagTakesNoValue(flag, value) => fmt.write_fmt(core::format_args!("Flag takes no value: --{}={}", flag, value.to_string_lossy())),
            Self::FlagToggleTakesNoValue(flag, value) => fmt.write_fmt(core::format_args!("Toggle-flag takes no value: --no-{}={}", flag, value.to_string_lossy())),
            Self::FlagNeedsValue(flag) => fmt.write_fmt(core::format_args!("Flag requires a value: --{}", flag)),
            Self::FlagSetValue(flag, e) => fmt.write_fmt(core::format_args!("Cannot parse value for flag `--{}`: {}", flag, e)),
            Self::FlagToggleValue(flag, v, e) => fmt.write_fmt(core::format_args!("Cannot parse value for toggle-flag `--[no-]{}={}`: {}", flag, v, e)),
            Self::FlagParseValue(flag, v, e) => fmt.write_fmt(core::format_args!("Cannot parse value for flag `--[no-]{}={}`: {}", flag, v.to_string_lossy(), e)),
            Self::ShortsUnknown(flags) => fmt.write_fmt(core::format_args!("Invalid short flags: {}", flags.to_string_lossy())),
            Self::CommandParameter(cmd, v, e) => fmt.write_fmt(core::format_args!("Cannot parse parameter for command `{} {}`: {}", cmd, v.to_string_lossy(), e)),
            Self::CommandTakesNoParameters(cmd, v) => fmt.write_fmt(core::format_args!("Invalid parameters for command: {} {}", cmd, v.to_string_lossy())),
        }
    }
}

// Allow creation of empty lists for all audited lists. This requires all
// implementors to allow empty lists without auditing.
impl<'a, T> Default for &'a AuditedList<[T]> {
    fn default() -> Self {
        &AuditedList::<[T; 0]> {
            list: [],
        }
    }
}

impl<'args, 'ctx, Id> Flag<'args, 'ctx, Id> {
    fn with(
        name: &'ctx str,
        value: Value<'args, Id>,
        help_short: Option<&'ctx str>,
    ) -> Self {
        Self {
            name: name,
            value: value,

            help_short: help_short,
        }
    }

    /// Create a command-line flag definition with the specified name and value
    /// location. All other properties of the flag will assume their defaults.
    pub fn with_name(
        name: &'ctx str,
        value: Value<'args, Id>,
        help_short: Option<&'ctx str>,
    ) -> Self {
        Self::with(name, value, help_short)
    }
}

impl<'args, 'ctx, const N: usize, Id> FlagList<'args, 'ctx, N, Id> {
    /// Create an audited list of command-line flags from user configuration.
    /// This will sort the flag array by their names and thus allow faster
    /// searches.
    pub fn with(mut list: [Flag<'args, 'ctx, Id>; N]) -> Self {
        list.sort_unstable_by_key(|v| v.name);
        Self {
            list: list,
        }
    }
}

impl<'args, 'ctx, Id> Command<'args, 'ctx, Id> {
    fn with(
        id: Id,
        name: &'ctx str,
        commands: CommandListRef<'args, 'ctx, Id>,
        flags: FlagListRef<'args, 'ctx, Id>,
        parameters: Option<Parameters<'args, Id>>,
        help_short: Option<&'ctx str>,
    ) -> Self {
        Self {
            id: id,
            name: name,
            commands: commands,
            flags: flags,
            parameters: parameters,
            help_short: help_short,
        }
    }

    /// Create a command-line command definition with the specified name,
    /// sub-commands, flags, and parameter parser. All other properties of the
    /// command will assume their defaults.
    pub fn with_name(
        id: Id,
        name: &'ctx str,
        commands: CommandListRef<'args, 'ctx, Id>,
        flags: FlagListRef<'args, 'ctx, Id>,
        parameters: Option<Parameters<'args, Id>>,
        help_short: Option<&'ctx str>,
    ) -> Self {
        Self::with(id, name, commands, flags, parameters, help_short)
    }

    fn find_command(
        &self,
        name: &str,
    ) -> Option<&'ctx Command<'args, 'ctx, Id>> {
        match self.commands.list.binary_search_by_key(
            &name,
            |v| v.name,
        ) {
            Ok(v) => Some(&self.commands.list[v]),
            _ => None,
        }
    }

    fn find_flag(
        &self,
        name: &str,
    ) -> Option<&'ctx Flag<'args, 'ctx, Id>> {
        match self.flags.list.binary_search_by_key(
            &name,
            |v| v.name,
        ) {
            Ok(v) => Some(&self.flags.list[v]),
            _ => None,
        }
    }

    /// Write usage information to the specified format stream. This will
    /// include short explanations for the individual items.
    ///
    /// Only information for the current level will be printed.
    pub fn help(
        &self,
        dst: &mut dyn core::fmt::Write,
        trace: &alloc::vec::Vec<&'ctx Command<'args, 'ctx, Id>>,
    ) -> Result<(), core::fmt::Error> {
        // Start with one-line description.
        if let Some(v) = self.help_short {
            dst.write_fmt(core::format_args!("{}\n\n", v))?;
        }

        // Follow with usage information.
        dst.write_str("Usage:")?;
        for cmd in trace {
            dst.write_fmt(core::format_args!(" {}", cmd.name))?;
        }
        let usage = match (
            self.flags.list.len() > 0,
            self.commands.list.len() > 0,
        ) {
            (false, false) => "",
            (false, true) => " <COMMAND>",
            (true, false) => " [OPTIONS]",
            (true, true) => " [OPTIONS] <COMMAND>",
        };
        dst.write_fmt(core::format_args!(" {}{}\n", self.name, usage))?;

        // List all options for this level.
        let mut flags = self.flags.list.iter()
            .filter(|v| v.help_short.is_some())
            .peekable();
        if flags.peek().is_some() {
            dst.write_str("\nOptions:\n")?;

            let maxlen = flags.clone()
                .map(|v| v.name.len())
                .max()
                .unwrap();

            for flag in flags {
                dst.write_fmt(core::format_args!(
                    "    --{0:1$}  {2}\n",
                    flag.name,
                    maxlen,
                    flag.help_short.unwrap(),
                ))?;
            }
        }

        // List all commands for this level.
        let mut cmds = self.commands.list.iter()
            .filter(|v| v.help_short.is_some())
            .peekable();
        if cmds.peek().is_some() {
            dst.write_str("\nCommands:\n")?;

            let maxlen = cmds.clone()
                .map(|v| v.name.len())
                .max()
                .unwrap();

            for cmd in cmds {
                dst.write_fmt(core::format_args!(
                    "    {0:1$}  {2}\n",
                    cmd.name,
                    maxlen,
                    cmd.help_short.unwrap(),
                ))?;
            }
        }

        Ok(())
    }
}

impl<'args, 'ctx, const N: usize, Id> CommandList<'args, 'ctx, N, Id> {
    /// Create an audited list of command-line sub-commands from user
    /// configuration. This will sort the sub-command array by their names and
    /// thus allow faster searches.
    pub fn with(mut list: [Command<'args, 'ctx, Id>; N]) -> Self {
        list.sort_unstable_by_key(|v| v.name);
        Self {
            list: list,
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
    ) -> Option<&'ctx Flag<'args, 'ctx, Id>> {
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
        current: &'ctx Command<'args, 'ctx, Id>,
        history: &alloc::vec::Vec<&'ctx Command<'args, 'ctx, Id>>,
        flag_str: &'args str,
        value_opt: Option<&'args compat::OsStr>,
    ) -> Result<(), Error<'args>>
    where
        Id: Clone,
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

        match (&flag.value, flag_toggled, value_opt) {
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
                s.push(current.id.clone(), ()).map_err(
                    |e| Error::FlagSetValue(flag_str, e),
                )
            },
            (Value::Toggle(s), t, None) => {
                // Correct use of toggle-flag.
                s.push(current.id.clone(), t.is_none()).map_err(
                    |e| Error::FlagToggleValue(t.unwrap_or(flag_str), t.is_none(), e),
                )
            },
            (Value::Parse(s), None, None) => {
                // Flag requires a value, so fetch it.
                match arguments.next() {
                    None => Err(Error::FlagNeedsValue(flag_str)),
                    Some(v) => s.push(current.id.clone(), v).map_err(
                        |e| Error::FlagParseValue(flag_str, v, e),
                    )
                }
            },
            (Value::Parse(s), None, Some(v)) => {
                // Flag requires a value that was passed inline.
                s.push(current.id.clone(), v).map_err(
                    |e| Error::FlagParseValue(flag_str, v, e),
                )
            },
        }
    }

    fn parse_short<'args, 'ctx, Id>(
        &mut self,
        _history: &alloc::vec::Vec<&'ctx Command<'args, 'ctx, Id>>,
        short_str: &'args compat::OsStr,
    ) -> Result<(), Error<'args>> {
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
    ) -> Result<Option<&'ctx Command<'args, 'ctx, Id>>, Error<'args>>
    where
        Id: Clone,
    {
        let sub_opt = match arg_str_opt {
            None => None,
            Some(arg_str) => command.find_command(arg_str),
        };

        if let Some(sub) = sub_opt {
            Ok(Some(sub))
        } else if let Some(ref v) = command.parameters {
            v.push(command.id.clone(), arg_os).map_err(
                |e| Error::CommandParameter(command.name.into(), arg_os, e),
            )?;
            Ok(None)
        } else {
            Err(Error::CommandTakesNoParameters(command.name.into(), arg_os))
        }
    }

    fn parse_root<'args, 'ctx, Id, Source>(
        &mut self,
        mut arguments: Source,
        command: &'ctx Command<'args, 'ctx, Id>,
    ) -> Result<Id, alloc::boxed::Box<[Error<'args>]>>
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
                        if let Some(ref p) = current.parameters {
                            while let Some(v) = arguments.next() {
                                if let Err(e) = p.push(current.id.clone(), v) {
                                    errors.push(Error::CommandParameter(
                                        current.name.into(), v, e,
                                    ));
                                }
                            }
                        } else if let Some(v) = arguments.next() {
                            errors.push(Error::CommandTakesNoParameters(current.name.into(), v));
                        }
                    },

                    (_, false, _) => {
                        // We got a complete flag with or without value. Look
                        // up the flag and pass the value along, if required.
                        if let Err(e) = self.parse_flag(&mut arguments, &current, &history, flag, value) {
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
    ) -> Result<Id, alloc::boxed::Box<[Error<'args>]>>
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
    ) -> Result<Id, alloc::boxed::Box<[Error<'args>]>>
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
    ) -> Result<Id, alloc::boxed::Box<[Error<'args>]>>
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

impl<Id> Help<Id>
where
    Id: PartialEq,
{
    /// Create a new context for handling of common `--help` arguments.
    pub fn new() -> Self {
        Self {
            cell: core::cell::RefCell::new(None),
        }
    }

    /// Try handling any `--help` arguments. This will return `true` if this
    /// command-line flag was set, otherwise `false` is returned. Furthermore,
    /// if set, it will write respective usage information to the specified
    /// destination.
    pub fn help_for<'args, 'ctx>(
        command: &'ctx Command<'args, 'ctx, Id>,
        dst: &mut dyn core::fmt::Write,
        id: &Id,
    ) -> Result<bool, core::fmt::Error> {
        let mut trace = alloc::vec::Vec::new();
        let mut todo = alloc::vec::Vec::new();

        todo.push(Some(command));

        // Traverse the tree of sub-commands, keeping a trace so the
        // help-handler can utilize the chain.
        while let Some(o_current) = todo.pop() {
            if let Some(current) = o_current {
                if *id == current.id {
                    current.help(dst, &trace)?;
                    return Ok(true);
                }

                if current.commands.list.len() > 0 {
                    trace.push(current);
                    todo.push(None);
                    for cmd in current.commands.list.iter() {
                        todo.push(Some(cmd));
                    }
                }
            } else {
                trace.pop();
            }
        }

        Ok(false)
    }

    /// Try handling any `--help` arguments. This will return `true` if this
    /// command-line flag was set, otherwise `false` is returned. Furthermore,
    /// if set, it will write respective usage information to the specified
    /// destination.
    pub fn help<'args, 'ctx>(
        &self,
        command: &'ctx Command<'args, 'ctx, Id>,
        dst: &mut dyn core::fmt::Write,
    ) -> Result<bool, core::fmt::Error> {
        if let Some(ref id) = *self.cell.borrow() {
            Self::help_for(command, dst, id)
        } else {
            Ok(false)
        }
    }
}

impl<Id> sink::Sink<Id, ()> for Help<Id>
where
    Id: core::fmt::Debug,
{
    fn push(
        &self,
        ctx: Id,
        _data: (),
    ) -> Result<(), sink::Error> {
        self.cell.replace(Some(ctx));
        Ok(())
    }
}

pub mod sink {
    //! # Interfaces for Generic Data Sinks
    //!
    //! Data sinks allow generalizing the way how data is collected or stored.
    //! The `SinkMut` trait defines how any type can accept specific input data
    //! and store it, possibly raising errors if the data could not be parsed.
    //! The `Sink` trait defines a variant for sinks with interior mutability.

    use crate::compat;

    /// Enumeration of errors that can be raised by data sinks. The enumeration
    /// is not exhaustive and uncaught errors must be handled by callers.
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Error {
        /// Value was not valid for this data parser
        ValueInvalid,
        /// Data was not encoded as valid Unicode
        UnicodeInvalid,
    }

    /// Generic data sink with inherited mutability. It defines how data is
    /// collected and stored, providing a uniform interface to the caller.
    /// Sinks are specific to the type of the source data, and can be
    /// implemented for a wide range of different sources.
    pub trait SinkMut<Context, Source>
    where
        Self: core::fmt::Debug,
    {
        /// Push data into the sink, reporting whether it was stored
        /// successfully. Usually, this requires the implementor to parse the
        /// input data (if necessary) and then store it.
        ///
        /// It is up to the implementor to decide whether new data overrides
        /// old data, or whether it is amended.
        fn push(
            &mut self,
            ctx: Context,
            data: Source,
        ) -> Result<(), Error>;
    }

    /// Generic data sink with interior mutability. It defines how data is
    /// collected and stored, providing a uniform interface to the caller.
    /// Sinks are specific to the type of the source data, and can be
    /// implemented for a wide range of different sources.
    pub trait Sink<Context, Source>
    where
        Self: core::fmt::Debug,
    {
        /// Push data into the sink, reporting whether it was stored
        /// successfully. Usually, this requires the implementor to parse the
        /// input data (if necessary) and then store it.
        ///
        /// It is up to the implementor to decide whether new data overrides
        /// old data, or whether it is amended.
        fn push(
            &self,
            ctx: Context,
            data: Source,
        ) -> Result<(), Error>;
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, fmt: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
            match self {
                Self::ValueInvalid => fmt.write_str("Value is not valid"),
                Self::UnicodeInvalid => fmt.write_str("Value is not valid Unicode"),
            }
        }
    }

    impl<'args, Context> SinkMut<Context, bool> for bool {
        fn push(
            &mut self,
            _ctx: Context,
            data: bool,
        ) -> Result<(), Error> {
            *self = data;
            Ok(())
        }
    }

    impl<'args, Context> SinkMut<Context, &'args compat::OsStr> for &'args compat::OsStr {
        fn push(
            &mut self,
            _ctx: Context,
            data: &'args compat::OsStr,
        ) -> Result<(), Error> {
            *self = data;
            Ok(())
        }
    }

    #[cfg(feature = "std")]
    impl<'args, Context> SinkMut<Context, &'args compat::OsStr> for &'args std::ffi::OsStr {
        fn push(
            &mut self,
            _ctx: Context,
            data: &'args compat::OsStr,
        ) -> Result<(), Error> {
            *self = data.as_osstr();
            Ok(())
        }
    }

    #[cfg(feature = "std")]
    impl<'args, Context> SinkMut<Context, &'args compat::OsStr> for std::ffi::OsString {
        fn push(
            &mut self,
            _ctx: Context,
            data: &'args compat::OsStr,
        ) -> Result<(), Error> {
            *self = data.as_osstr().into();
            Ok(())
        }
    }

    impl<'args, Context> SinkMut<Context, &'args compat::OsStr> for &'args str {
        fn push(
            &mut self,
            _ctx: Context,
            data: &'args compat::OsStr,
        ) -> Result<(), Error> {
            if let Ok(data_str) = data.to_str() {
                *self = data_str;
                Ok(())
            } else {
                Err(Error::UnicodeInvalid)
            }
        }
    }

    impl<'args, Context> SinkMut<Context, &'args compat::OsStr> for alloc::string::String {
        fn push(
            &mut self,
            _ctx: Context,
            data: &'args compat::OsStr,
        ) -> Result<(), Error> {
            if let Ok(data_str) = data.to_str() {
                *self = data_str.into();
                Ok(())
            } else {
                Err(Error::UnicodeInvalid)
            }
        }
    }

    impl<'args, Context> SinkMut<Context, &'args compat::OsStr> for bool {
        fn push(
            &mut self,
            _ctx: Context,
            data: &'args compat::OsStr,
        ) -> Result<(), Error> {
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
                        Err(Error::ValueInvalid)
                    },
                }
            } else {
                Err(Error::UnicodeInvalid)
            }
        }
    }

    impl<Context, Source, Target> SinkMut<Context, Source> for Option<Target>
    where
        Target: SinkMut<Context, Source> + Default,
    {
        fn push(
            &mut self,
            ctx: Context,
            data: Source,
        ) -> Result<(), Error> {
            self.get_or_insert_with(Default::default)
                .push(ctx, data)
        }
    }

    impl<Context, Source, Target> SinkMut<Context, Source> for alloc::vec::Vec<Target>
    where
        Target: SinkMut<Context, Source> + Default,
    {
        fn push(
            &mut self,
            ctx: Context,
            data: Source,
        ) -> Result<(), Error> {
            let mut v: Target = Default::default();
            let r = v.push(ctx, data)?;
            self.push(v);
            Ok(r)
        }
    }

    impl<Context, Source, Target> Sink<Context, Source> for core::cell::RefCell<Target>
    where
        Target: SinkMut<Context, Source>,
    {
        fn push(
            &self,
            ctx: Context,
            data: Source,
        ) -> Result<(), Error> {
            self.borrow_mut().push(ctx, data)
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
        foo: core::cell::RefCell<Option<String>>,
        bar: core::cell::RefCell<Option<String>>,
        foofoo: core::cell::RefCell<Option<String>>,
        foobar: core::cell::RefCell<Option<String>>,
        barfoo: core::cell::RefCell<Option<String>>,
        barbar: core::cell::RefCell<Option<String>>,
    }

    fn parse<'args>(
        arguments: &'args [&'args str],
        values: &'args mut Values,
    ) -> Result<Id, alloc::boxed::Box<[Error<'args>]>> {
        let flags_foo = FlagList::with([
            Flag::with_name("foofoo", Value::Parse(&mut values.foofoo), None),
            Flag::with_name("foobar", Value::Parse(&mut values.foobar), None),
        ]);
        let flags_bar = FlagList::with([
            Flag::with_name("barfoo", Value::Parse(&mut values.barfoo), None),
            Flag::with_name("barbar", Value::Parse(&mut values.barbar), None),
        ]);
        let cmds = CommandList::with([
            Command::with_name(Id::Foo, "foo", Default::default(), &flags_foo, None, None),
            Command::with_name(Id::Bar, "bar", Default::default(), &flags_bar, None, None),
        ]);
        let flags = FlagList::with([
            Flag::with_name("foo", Value::Parse(&mut values.foo), None),
            Flag::with_name("bar", Value::Parse(&mut values.bar), None),
        ]);
        let cmd = Command::with_name(Id::Root, "cmd", &cmds, &flags, None, None);
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
                foo: core::cell::RefCell::new(Some("value-foo".into())),
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
                foo: core::cell::RefCell::new(Some("value-foo".into())),
                bar: core::cell::RefCell::new(Some("value-bar".into())),
                barbar: core::cell::RefCell::new(Some("value-barbar".into())),
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
            Error::CommandTakesNoParameters(ref v, _) if v == "cmd",
        ));

        let r = parse(
            &["foo", "invalid"],
            &mut values,
        ).unwrap_err();
        assert_eq!(r.len(), 1);
        assert!(core::matches!(
            r[0],
            Error::CommandTakesNoParameters(ref v, _) if v == "foo",
        ));
    }
}
