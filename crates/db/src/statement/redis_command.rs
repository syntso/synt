use super::Classification;
use crate::Effect;

const READ: [&str; 92] = [
    "BITCOUNT",
    "BITFIELD_RO",
    "BITPOS",
    "DBSIZE",
    "DUMP",
    "ECHO",
    "EXISTS",
    "EXPIRETIME",
    "GEODIST",
    "GEOHASH",
    "GEOPOS",
    "GEORADIUSBYMEMBER_RO",
    "GEORADIUS_RO",
    "GEOSEARCH",
    "GET",
    "GETBIT",
    "GETRANGE",
    "HEXISTS",
    "HGET",
    "HGETALL",
    "HKEYS",
    "HLEN",
    "HMGET",
    "HRANDFIELD",
    "HSCAN",
    "HSTRLEN",
    "HVALS",
    "INFO",
    "KEYS",
    "LASTSAVE",
    "LCS",
    "LINDEX",
    "LLEN",
    "LPOS",
    "LRANGE",
    "MGET",
    "PEXPIRETIME",
    "PFCOUNT",
    "PING",
    "PTTL",
    "RANDOMKEY",
    "SCAN",
    "SCARD",
    "SDIFF",
    "SINTER",
    "SINTERCARD",
    "SISMEMBER",
    "SMEMBERS",
    "SMISMEMBER",
    "SORT_RO",
    "SRANDMEMBER",
    "SSCAN",
    "STRLEN",
    "SUBSTR",
    "SUNION",
    "TIME",
    "TOUCH",
    "TTL",
    "TYPE",
    "XLEN",
    "XPENDING",
    "XRANGE",
    "XREAD",
    "XREVRANGE",
    "ZCARD",
    "ZCOUNT",
    "ZDIFF",
    "ZINTER",
    "ZINTERCARD",
    "ZLEXCOUNT",
    "ZMSCORE",
    "ZRANDMEMBER",
    "ZRANGE",
    "ZRANGEBYLEX",
    "ZRANGEBYSCORE",
    "ZRANK",
    "ZREVRANGE",
    "ZREVRANGEBYLEX",
    "ZREVRANGEBYSCORE",
    "ZREVRANK",
    "ZSCAN",
    "ZSCORE",
    "ZUNION",
    "HTTL",
    "HPTTL",
    "HEXPIRETIME",
    "HPEXPIRETIME",
    "LOLWUT",
    "OBJECT ENCODING",
    "OBJECT FREQ",
    "OBJECT IDLETIME",
    "OBJECT REFCOUNT",
];

const READ_CONTAINERS: [&str; 5] = [
    "MEMORY USAGE",
    "XINFO STREAM",
    "XINFO GROUPS",
    "XINFO CONSUMERS",
    "COMMAND COUNT",
];

const WRITE: [&str; 98] = [
    "APPEND",
    "BITFIELD",
    "BITOP",
    "BLMOVE",
    "BLMPOP",
    "BLPOP",
    "BRPOP",
    "BRPOPLPUSH",
    "BZMPOP",
    "BZPOPMAX",
    "BZPOPMIN",
    "COPY",
    "DECR",
    "DECRBY",
    "DEL",
    "EXPIRE",
    "EXPIREAT",
    "GEOADD",
    "GEORADIUS",
    "GEORADIUSBYMEMBER",
    "GEOSEARCHSTORE",
    "GETDEL",
    "GETEX",
    "GETSET",
    "HDEL",
    "HINCRBY",
    "HINCRBYFLOAT",
    "HMSET",
    "HSET",
    "HSETNX",
    "INCR",
    "INCRBY",
    "INCRBYFLOAT",
    "LINSERT",
    "LMOVE",
    "LMPOP",
    "LPOP",
    "LPUSH",
    "LPUSHX",
    "LREM",
    "LSET",
    "LTRIM",
    "MSET",
    "MSETNX",
    "PERSIST",
    "PEXPIRE",
    "PEXPIREAT",
    "PFADD",
    "PFMERGE",
    "PSETEX",
    "PUBLISH",
    "RENAME",
    "RENAMENX",
    "RPOP",
    "RPOPLPUSH",
    "RPUSH",
    "RPUSHX",
    "SADD",
    "SDIFFSTORE",
    "SET",
    "SETBIT",
    "SETEX",
    "SETNX",
    "SETRANGE",
    "SINTERSTORE",
    "SMOVE",
    "SORT",
    "SPOP",
    "SREM",
    "SUNIONSTORE",
    "UNLINK",
    "XACK",
    "XADD",
    "XAUTOCLAIM",
    "XCLAIM",
    "XDEL",
    "XTRIM",
    "XSETID",
    "ZADD",
    "ZDIFFSTORE",
    "ZINCRBY",
    "ZINTERSTORE",
    "ZMPOP",
    "ZPOPMAX",
    "ZPOPMIN",
    "ZRANGESTORE",
    "ZREM",
    "ZREMRANGEBYLEX",
    "ZREMRANGEBYRANK",
    "ZREMRANGEBYSCORE",
    "ZUNIONSTORE",
    "HEXPIRE",
    "HPEXPIRE",
    "HEXPIREAT",
    "HPEXPIREAT",
    "HPERSIST",
    "HGETDEL",
    "HGETEX",
];

const CONTAINERS: [&str; 5] = ["OBJECT", "MEMORY", "XINFO", "COMMAND", "XGROUP"];

pub(super) fn classify<S: AsRef<str>>(words: &[S]) -> Classification {
    let Some(first) = words.first() else {
        return Classification::new(Effect::Privileged, "empty");
    };

    let name = first.as_ref().trim().to_ascii_uppercase();

    if CONTAINERS.contains(&name.as_str()) {
        return container(&name, words.get(1).map(AsRef::as_ref));
    }

    Classification::new(effect(&name), name.to_ascii_lowercase())
}

fn container(name: &str, sub: Option<&str>) -> Classification {
    let full = match sub {
        Some(sub) => format!("{name} {}", sub.trim().to_ascii_uppercase()),
        None => name.to_owned(),
    };

    let effect = if READ.contains(&full.as_str()) || READ_CONTAINERS.contains(&full.as_str()) {
        Effect::Read
    } else if full.starts_with("XGROUP ") {
        Effect::Write
    } else {
        Effect::Privileged
    };

    Classification::new(effect, full.to_ascii_lowercase())
}

fn effect(name: &str) -> Effect {
    if READ.contains(&name) {
        return Effect::Read;
    }

    if WRITE.contains(&name) {
        return Effect::Write;
    }

    Effect::Privileged
}

#[cfg(test)]
mod tests {
    use super::classify;
    use crate::Effect;

    fn effect(words: &[&str]) -> Effect {
        classify(words).effect
    }

    #[test]
    fn lookups_are_reads_in_any_letter_case() {
        assert_eq!(effect(&["GET", "user:1"]), Effect::Read);
        assert_eq!(effect(&["hgetall", "user:1"]), Effect::Read);
        assert_eq!(effect(&["Scan", "0", "MATCH", "user:*"]), Effect::Read);
        assert_eq!(effect(&["PING"]), Effect::Read);
    }

    #[test]
    fn changes_to_keys_are_writes() {
        assert_eq!(effect(&["SET", "user:1", "x"]), Effect::Write);
        assert_eq!(effect(&["DEL", "user:1"]), Effect::Write);
        assert_eq!(effect(&["LPUSH", "queue", "job"]), Effect::Write);
        assert_eq!(effect(&["XGROUP", "CREATE", "s", "g", "$"]), Effect::Write);
    }

    #[test]
    fn server_control_and_scripts_are_privileged() {
        for words in [
            vec!["FLUSHALL"],
            vec!["FLUSHDB"],
            vec!["CONFIG", "GET", "requirepass"],
            vec!["EVAL", "return 1", "0"],
            vec!["SHUTDOWN"],
            vec!["SELECT", "1"],
            vec!["MULTI"],
            vec!["ACL", "LIST"],
        ] {
            assert_eq!(effect(&words), Effect::Privileged, "{words:?}");
        }
    }

    #[test]
    fn an_unknown_command_is_privileged() {
        assert_eq!(effect(&["MADEUP", "x"]), Effect::Privileged);
    }

    #[test]
    fn subcommands_decide_for_container_commands() {
        assert_eq!(effect(&["OBJECT", "encoding", "k"]), Effect::Read);
        assert_eq!(effect(&["MEMORY", "USAGE", "k"]), Effect::Read);
        assert_eq!(effect(&["MEMORY", "PURGE"]), Effect::Privileged);
        assert_eq!(
            classify(&["object", "encoding", "k"]).kind,
            "object encoding"
        );
    }

    #[test]
    fn no_words_is_privileged() {
        let none: [&str; 0] = [];

        assert_eq!(classify(&none).effect, Effect::Privileged);
    }
}
