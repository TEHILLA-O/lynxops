# Namespaces

`lynxops inspect` reads `/proc/<pid>/ns/*` and prints the symlink targets. Those strings are namespace *identities*, not names.

```text
pid:[4026531836]   mnt:[4026531840]   net:[4026531992]
```

Two processes share a namespace when the inode numbers match.

## What we surface

| Link | Question it answers |
| --- | --- |
| `pid` | Is this a container PID namespace? Compare to pid 1. |
| `mnt` | Different mount table (containers, `unshare -m`, snaps). |
| `net` | Own network stack — sockets in `/proc/<pid>/net` not `/proc/net`. |
| `uts` | Own hostname. |
| `ipc` | Isolated SysV / POSIX IPC. |
| `user` | User namespace (root-in-container). |
| `cgroup` | cgroup namespace (what the process sees under `/sys/fs/cgroup`). |
| `time` | Time namespace (uncommon; empty on older kernels). |

## Incident notes

- A process whose `net` inode differs from pid 1 is almost certainly in a container or a `ip netns` namespace. LynxOps still reads the *host* `/proc/net` tables; sockets inside another netns appear under that pid’s `/proc/<pid>/net/*`. Joining those tables is a follow-up (`inspect` currently joins host tables via fd inodes, which still works: the inode is global).
- `user` != pid 1’s `user` plus `Uid: 0` in `status` is the usual “root in a user ns” pattern.
- Nested PID namespaces show `NSpid` in `status` (host pid, then inner). We do not parse `NSpid` yet; the `pid` ns inode is enough to see isolation.

## Privileges

`/proc/<pid>/ns` is readable for the owner’s processes. Other users get `EACCES`; inspect then leaves those fields empty.
