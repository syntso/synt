use crate::{Classification, Effect};

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

pub(super) struct RedisCommand {
    name: String,
    container: bool,
}

impl RedisCommand {
    pub(super) fn parse<S: AsRef<str>>(words: &[S]) -> Option<Self> {
        let (first, rest) = words.split_first()?;
        let name = first.as_ref().trim().to_ascii_uppercase();

        if !CONTAINERS.contains(&name.as_str()) {
            return Some(Self {
                name,
                container: false,
            });
        }

        let name = match rest.first() {
            Some(sub) => format!("{name} {}", sub.as_ref().trim().to_ascii_uppercase()),
            None => name,
        };

        Some(Self {
            name,
            container: true,
        })
    }

    pub(super) fn classification(&self) -> Classification {
        Classification::new(self.effect(), self.name.to_ascii_lowercase())
    }

    fn effect(&self) -> Effect {
        let name = self.name.as_str();

        if READ.contains(&name) || (self.container && READ_CONTAINERS.contains(&name)) {
            return Effect::Read;
        }

        let writes = if self.container {
            name.starts_with("XGROUP ")
        } else {
            WRITE.contains(&name)
        };

        if writes {
            return Effect::Write;
        }

        Effect::Privileged
    }
}

#[cfg(test)]
mod tests {
    use crate::{Classification, Effect};

    fn classify(words: &[&str]) -> Classification {
        Classification::of_command(words)
    }

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
