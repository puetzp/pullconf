# Logging

## Server

The server logs from **pullconfd** are pretty straightforward. By default systemd redirects `stdout` and `stderr` to `/var/log/pullconfd/pullconfd.log`.

The `deb` package comes with a default logrotate configuration that causes logs to be rotated weekly.

The server is not very chatty as the most interesting part on the server side are errors. Most errors will come from being unable to parse invalid StrictYAML configuration. The server will fail on the first error that it encounters while validating the configuration or environment variables. So even when there are multiple issues with the configuration, the log will only show the first.

The log is supposed to be human-readable and not optimized for parsing.

## Client

The client logs from **pullconf** are more comprehensive, since the really interesting stuff happens on the client side. Systemd splits the output from **pullconf** into two files:

- `stderr` is redirected to `/var/log/pullconf/pullconf.log`.
- `stdout` is redirected to `/var/log/pullconf/resources.json.log`

The `deb` package comes with a default logrotate configuration that causes both logs to be rotated daily.

The first log file is supposed to print useful information and errors in a human-readable format, just as the server log from **pullconfd**.

The second log file is optimized for further processing and contains more detailed information about how the resources were applied. As the name indicates it is formatted as JSON. Each log line is a JSON document that corresponds to a **pullconf** process. In other words, when **pullconf** applied all resources it also prints a JSON report before exiting. So the last log line contains the results of the most recent invocation of `pullconf.service`.

While the first log usually tells the user what went wrong with a particular resource, the JSON log is designed to be used for debugging. As such it also contains information about the relationships between resources that were either explicitly requested by the user or implicitly formed by **pullconfd**.

The following is an example of the content of a single log line from `/var/log/pullconf/resources.json.log`.

```sh
tail -n1 /var/log/pullconf/resources.json.log | jq
```

```json
{
  "pid": 32199,
  "timestamp": 1740845030179,
  "duration": 0,
  "resources": [
    {
      "type": "file",
      "id": "f6caa473b7094924d4de4cdb7f178a103e3e65b23bac2f828ed7a70284b814b5",
      "parameters": {
        "path": "/tmp/testfile"
      },
      "relationships": {
        "requires": [],
        "after": [],
        "triggers": [
          {
            "type": "execute",
            "id": "4bd3fe3146d9f67d4e1dfff76e457dea60306734194eedc2608c0f5bac958778",
            "when": [
              "created",
              "deleted",
              "changed"
            ]
          }
        ]
      },
      "result": {
        "order": 0,
        "state": "unchanged",
        "message": null,
        "duration": 0
      }
    },
    {
      "type": "file",
      "id": "ecd155981b4d9689703361be0d6d19723ec90e2e88dfa0004b854d2dab4b2ed6",
      "parameters": {
        "path": "/tmp/another-testfile"
      },
      "relationships": {
        "requires": [
          {
            "type": "file",
            "id": "f6caa473b7094924d4de4cdb7f178a103e3e65b23bac2f828ed7a70284b814b5"
          }
        ],
        "after": [
          {
            "type": "file",
            "id": "f6caa473b7094924d4de4cdb7f178a103e3e65b23bac2f828ed7a70284b814b5"
          }
        ],
        "triggers": [
          {
            "type": "execute",
            "id": "4bd3fe3146d9f67d4e1dfff76e457dea60306734194eedc2608c0f5bac958778",
            "when": [
              "deleted"
            ]
          }
        ]
      },
      "result": {
        "order": 1,
        "state": "failed",
        "message": "failed to download file: server failed to process the request: 404 Not Found",
        "duration": 0
      }
    },
    {
      "type": "execute",
      "id": "4bd3fe3146d9f67d4e1dfff76e457dea60306734194eedc2608c0f5bac958778",
      "parameters": {
        "name": "do-stuff"
      },
      "relationships": {
        "requires": [],
        "after": [
          {
            "type": "file",
            "id": "f6caa473b7094924d4de4cdb7f178a103e3e65b23bac2f828ed7a70284b814b5"
          },
          {
            "type": "file",
            "id": "ecd155981b4d9689703361be0d6d19723ec90e2e88dfa0004b854d2dab4b2ed6"
          }
        ],
        "triggers": []
      },
      "result": {
        "order": 2,
        "state": "unchanged",
        "message": null,
        "duration": 0
      }
    }
  ]
}
```

- `pid` is the process ID of the **pullconf** process that produced this log line.
- `timestamp` is a UNIX timestamp, the number of milliseconds that elapsed since the UNIX epoch.
- `duration` is the total number of milliseconds that it took to apply all resources.
- each item in `resources` contains information about the state of the resource after it has been applied:
  - `type` and the primary parameter of the given type in `parameters` identify the resource.
  - `result.order` is the position at which the resource was applied, starting from zero.
  - `result.state` is the final state of the resource after it was applied. In most cases, when there is nothing to do, this will be `unchanged`.
  - `result.message` contains hints as to why a resource could not be applied, or why it might have been skipped, if that is the case.
  - `result.duration` is the number of milliseconds it took to apply this resource.

As already mentioned the log is optimized for further processing. As such `timestamp` contains a UNIX timestamp instead of a human-readable date format. For ad-hoc debugging this can be circumvented by applying a filter in `jq`:

```sh
tail -n1 /var/log/pullconf/resources.json.log| jq '.timestamp |=  (tonumber / 1000 | todate)'
```

This converts `timestamp` to an iso8601 string.
