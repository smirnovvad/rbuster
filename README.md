Rbuster v0.1.1
========================================
yet another dirbuster
### Common Command line options
* `-a <user agent string>` - specify a user agent string to send in the request header.
* `-r` - follow redirects.
* `-s <status codes>` - comma-separated set of the list of status codes to be deemed a "positive" (default: `200,204,301,302,307`).
* `-u <url/domain>` - full URL (including scheme), or base domain name.
* `-v` - verbose output (show all results).
* `-w <wordlist>` - path to the wordlist used for brute forcing.

### Install
```
cargo install rbuster
```

### Example
```
$ rbuster -w common.txt -u http://horriblesubs.info/

Rbuster 0.1.0                 Vadim Smirnov
=====================================================
Url/Domain    : http://horriblesubs.info/
Wordlist      : common.txt
Words         : 4593
=====================================================
/thanks (Status: 301 Moved Permanently | Content-Length: 0)
/the (Status: 301 Moved Permanently | Content-Length: 0)
/ro (Status: 301 Moved Permanently | Content-Length: 0)
/robot (Status: 301 Moved Permanently | Content-Length: 0)
/robotics (Status: 301 Moved Permanently | Content-Length: 0)
/robots.txt (Status: 200 OK | Content-Length: 67)
```
