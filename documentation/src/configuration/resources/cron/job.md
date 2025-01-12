# cron::job

This resource manages a crontab file in the directory `/etc/cron.d`. A file created by this resource contains only a single command. Unlike user-owned crontabs, files in `/etc/cron.d` are parsed by `cron` as system-wide crontabs which may be run as any user. As such the cron::job resource also provides a `user` parameter.

For more information about cron and crontabs see `man cron` and `man 5 crontab`.

## Relationship to other resources

This resource implicitly depends on [directory](../directory.md) and [symlink](../symlink.md) resources whose `path` parameters are ancestors to the directory `/etc/cron.d`.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `name` | string | *Primary parameter*: Name of the cron job. The name will also be used as file name. Note that `cron` ignores files whose names contain dots and certain other characters. Allowed characters are: `[a-zA-Z0-9_\-]` | yes | |
| `schedule` | string | A schedule expression that indicates when `cron` should execute the job. | yes | |
| `user` | string | The name of the user that the command should be run as. | yes | |
| `command` | string | The command to execute. | yes | |
| `environment` | array | Environment variables that the process run by `cron` should inherhit. | no | |

The `environment` array must contain hashes with the following keys:

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `name` | string | The name of the environment variable. | yes | |
| `value` | string | The value of the environment variable. If this parameter is omitted the variable will be set to an empty string. | no | |

## Examples

```yaml
resources:
  - type: cron::job
    parameters:
	  name: some-job
	  environment:
	    - name: MAILTO
		- name: PATH
		  value: /bin:/usr/bin:/usr/local/sbin
	  schedule: @daily
	  user: root
	  command: run-me-daily.py
```

