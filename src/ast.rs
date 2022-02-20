#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused)]
#![allow(clippy::all)]

// Type aliases
pub type bits32 = u32;

// Generated types
include!(concat!(env!("OUT_DIR"), "/ast.rs"));

#[derive(Debug, serde::Deserialize)]
pub struct Value(pub Node);

impl Value {
    pub fn inner(&self) -> &Node {
        &self.0
    }
}

pub(crate) mod constants {
    // FrameOptions is an OR of these bits.  The NONDEFAULT and BETWEEN bits are
    // used so that ruleutils.c can tell which properties were specified and
    // which were defaulted; the correct behavioral bits must be set either way.
    // The START_foo and END_foo options must come in pairs of adjacent bits for
    // the convenience of gram.y, even though some of them are useless/invalid.
    /// any specified?
    pub const FRAMEOPTION_NONDEFAULT: i32 = 0x00001;
    /// RANGE behavior
    pub const FRAMEOPTION_RANGE: i32 = 0x00002;
    /// ROWS behavior
    pub const FRAMEOPTION_ROWS: i32 = 0x00004;
    /// GROUPS behavior
    pub const FRAMEOPTION_GROUPS: i32 = 0x00008;
    /// BETWEEN given?
    pub const FRAMEOPTION_BETWEEN: i32 = 0x00010;
    /// start is U. P.
    pub const FRAMEOPTION_START_UNBOUNDED_PRECEDING: i32 = 0x00020;
    /// (disallowed)
    pub const FRAMEOPTION_END_UNBOUNDED_PRECEDING: i32 = 0x00040;
    /// (disallowed)
    pub const FRAMEOPTION_START_UNBOUNDED_FOLLOWING: i32 = 0x00080;
    /// end is U. F.
    pub const FRAMEOPTION_END_UNBOUNDED_FOLLOWING: i32 = 0x00100;
    /// start is C. R.
    pub const FRAMEOPTION_START_CURRENT_ROW: i32 = 0x00200;
    /// end is C. R.
    pub const FRAMEOPTION_END_CURRENT_ROW: i32 = 0x00400;
    /// start is O. P.
    pub const FRAMEOPTION_START_OFFSET_PRECEDING: i32 = 0x00800;
    /// end is O. P.
    pub const FRAMEOPTION_END_OFFSET_PRECEDING: i32 = 0x01000;
    /// start is O. F.
    pub const FRAMEOPTION_START_OFFSET_FOLLOWING: i32 = 0x02000;
    /// end is O. F.
    pub const FRAMEOPTION_END_OFFSET_FOLLOWING: i32 = 0x04000;
    /// omit C.R.
    pub const FRAMEOPTION_EXCLUDE_CURRENT_ROW: i32 = 0x08000;
    /// omit C.R. & peers
    pub const FRAMEOPTION_EXCLUDE_GROUP: i32 = 0x10000;
    /// omit C.R.'s peers
    pub const FRAMEOPTION_EXCLUDE_TIES: i32 = 0x20000;

    pub const ATTRIBUTE_IDENTITY_ALWAYS: char = 'a';
    pub const ATTRIBUTE_IDENTITY_BY_DEFAULT: char = 'd';
    pub const ATTRIBUTE_GENERATED_STORED: char = 's';

    pub const DEFAULT_INDEX_TYPE: &str = "btree";

    pub const FKCONSTR_ACTION_NOACTION: char = 'a';
    pub const FKCONSTR_ACTION_RESTRICT: char = 'r';
    pub const FKCONSTR_ACTION_CASCADE: char = 'c';
    pub const FKCONSTR_ACTION_SETNULL: char = 'n';
    pub const FKCONSTR_ACTION_SETDEFAULT: char = 'd';

    /* Foreign key matchtype codes */
    pub const FKCONSTR_MATCH_FULL: char = 'f';
    pub const FKCONSTR_MATCH_PARTIAL: char = 'p';
    pub const FKCONSTR_MATCH_SIMPLE: char = 's';

    /* Internal codes for partitioning strategies */
    pub const PARTITION_STRATEGY_HASH: char = 'h';
    pub const PARTITION_STRATEGY_LIST: char = 'l';
    pub const PARTITION_STRATEGY_RANGE: char = 'r';

    /* default selection for replica identity (primary key or nothing) */
    pub const REPLICA_IDENTITY_DEFAULT: char = 'd';
    /* no replica identity is logged for this relation */
    pub const REPLICA_IDENTITY_NOTHING: char = 'n';
    /* all columns are logged as replica identity */
    pub const REPLICA_IDENTITY_FULL: char = 'f';
    /*
     * an explicitly chosen candidate key's columns are used as replica identity.
     * Note this will still be set if the index has been dropped; in that case it
     * has the same meaning as 'd'.
     */
    pub const REPLICA_IDENTITY_INDEX: char = 'i';

    pub mod interval {
        pub const MONTH: i64 = 2;
        pub const YEAR: i64 = 4;
        pub const DAY: i64 = 8;
        pub const HOUR: i64 = 1024;
        pub const MINUTE: i64 = 2048;
        pub const SECOND: i64 = 4096;
        pub const YEAR_MONTH: i64 = YEAR | MONTH;
        pub const DAY_HOUR: i64 = DAY | HOUR;
        pub const DAY_HOUR_MINUTE: i64 = DAY | HOUR | MINUTE;
        pub const DAY_HOUR_MINUTE_SECOND: i64 = DAY | HOUR | MINUTE | SECOND;
        pub const HOUR_MINUTE: i64 = HOUR | MINUTE;
        pub const HOUR_MINUTE_SECOND: i64 = HOUR | MINUTE | SECOND;
        pub const MINUTE_SECOND: i64 = MINUTE | SECOND;
        pub const FULL_RANGE: i64 = 0x7FFF;
        pub const FULL_PRECISION: i64 = 0xFFFF;
    }

    pub mod lock {
        pub const AccessShareLock: i32 = 1; /* SELECT */
        pub const RowShareLock: i32 = 2; /* SELECT FOR UPDATE/FOR SHARE */
        pub const RowExclusiveLock: i32 = 3; /* INSERT, UPDATE, DELETE */
        pub const ShareUpdateExclusiveLock: i32 = 4; /* VACUUM (non-FULL),ANALYZE, CREATE INDEX
                                                      * CONCURRENTLY */
        pub const ShareLock: i32 = 5; /* CREATE INDEX (WITHOUT CONCURRENTLY) */
        pub const ShareRowExclusiveLock: i32 = 6; /* like EXCLUSIVE MODE, but allows ROW
                                                   * SHARE */
        pub const ExclusiveLock: i32 = 7; /* blocks ROW SHARE/SELECT...FOR UPDATE */
        pub const AccessExclusiveLock: i32 = 8; /* ALTER TABLE, DROP TABLE, VACUUM FULL,
                                                 * and unqualified LOCK TABLE */
    }

    pub mod trigger {
        /* Bits within tgtype */
        pub const TRIGGER_TYPE_AFTER: i16 = 0;
        pub const TRIGGER_TYPE_ROW: i16 = (1 << 0);
        pub const TRIGGER_TYPE_BEFORE: i16 = (1 << 1);
        pub const TRIGGER_TYPE_INSERT: i16 = (1 << 2);
        pub const TRIGGER_TYPE_DELETE: i16 = (1 << 3);
        pub const TRIGGER_TYPE_UPDATE: i16 = (1 << 4);
        pub const TRIGGER_TYPE_TRUNCATE: i16 = (1 << 5);
        pub const TRIGGER_TYPE_INSTEAD: i16 = (1 << 6);
    }
}
