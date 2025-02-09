# execute

This resource executes a command when triggered by other resources through the `trigger` meta-parameter. See [triggers](../triggers.md) for more information.

## Relationship to other resources

An `execute` resource implicitly depends on every resource that references this resource through the `trigger` meta-parameter. This ensures that the `execute` resource runs after those resources.

For instance when a [file](file.md) resource specifies an `execute` resource through `trigger` and the primary `name` parameter of the `execute` resource, the resource will be triggered when the [file](file.md) resource is either created, deleted or changed. When other resources trigger the same `execute` resource, the resource is ensured to run only once after each of those resources were applied.

> Right now this resource is limited in that it runs only when triggerd by other resources.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `name` | string | *Primary parameter*: The resource name. | yes | |
| `command` | array | The command and its arguments as strings. The program name and its arguments are each separate array items. | yes | |
| `environment` | array | Environment variables that the process that executes the `command` should inherhit. | no | |

The `environment` array must contain hashes with the following keys:

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `name` | string | The name of the environment variable. | yes | |
| `value` | string | The value of the environment variable. If this parameter is omitted the variable will be set to an empty string. | no | |

> Note that `command` is not passed through a shell. One way to use shell-specific features such as pipes is to install a script (e.g. via [file](file.md)) first and then execute this script in `command`.

## Examples

```yaml
# This does nothing unless explicitly triggered by another resource.
resources:
  - type: execute
    parameters:
      name: reload
	  command:
	    - systemctl
	    - daemon-reload
```

```yaml
# A common use case: managing a configuration file and reloading
# a service when the file changes.
resources:
  - type: execute
    parameters:
	  ensure: present
      name: reload-sshd
	  command:
	    - systemctl
	    - reload
		- sshd.service

  - type: file
    parameters:
      ensure: present
      path: /etc/ssh/sshd_config
	  owner: root
	  group: root
	  content:
	    value: |
          Port 22
		  Listen 0.0.0.0
		  
		  PermitRootLogin no
		  PasswordAuthentication no
		  PubkeyAuthentication yes
		  AllowTcpForwarding yes
    triggers:
      - type: execute
        name: reload-sshd
```
