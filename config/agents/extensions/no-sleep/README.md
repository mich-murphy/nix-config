# Pi No Sleep

Prevents macOS idle and display sleep while Pi is doing agent work. It also
prevents system sleep while the Mac is connected to AC power.
Each active Pi process owns one `/usr/bin/caffeinate` process. The assertion
starts on `agent_start`, remains active across retries and compaction, and ends
on `agent_settled` or `session_shutdown`. One unexpected process exit gets a
single restart attempt; further failures are reported without a restart loop.

The extension has no settings or commands. Installing it enables this command:

```text
/usr/bin/caffeinate -d -i -s -w <pi-pid>
```

Other operating systems remain unchanged.
