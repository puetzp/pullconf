# file

This resource manages a file within the filesystem hierarchy of the client.

## Relationship to other resources

The value of the `path` parameter must be unique among all file, [directory](directory.md) and [symlink](symlink.md) resources.

A file implicitly depends on [directory](directory.md) and [symlink](symlink.md) resources whose `path` parameters are ancestors to the file's `path`. For example when the `path` parameter of a file is set to `/my/simple/path` and there is a directory resource whose `path` is `/my/simple`, then the former implicitly depends on the latter.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `path` | string | *Primary parameter*: An absolute filesystem path. | yes | |
| `mode` | string | The file permission mode in octal notation. | yes | `644` |
| `owner` | string | The name of the user who owns the file. | yes | `root` |
| `group` | string | The name of the group who owns the file. | no | |
| `source` | string | An absolute path to a file asset on the server. The remote file will be downloaded and its contents written to the file on the client. For example when this parameter is set to `/my/example/file` the file must exist on the server at `$PULLCONF_ASSET_DIR/my/example/file` in order to be downloaded successfully by the client. The downloaded file is not subject to variable substitution and will be written as-is. Ideal for binary or static text files. Mutually exclusive with `content`. | no | |
| `content` | hash | Sets the content that may optionally be substituted with server-side variables as well as client-side script output. Mutually exclusive with `source`. | no | |

The `content` hash must contain the following keys:

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `value` | string | A single- or multiline string that contains the designated file content. Variables (`${pullconf::<var>}`) inside the string are substituted during validation, like in other resource parameters. | yes | |
| `replace` | array | An array of commands that run on the client and replace placeholders inside `value`. Commands are run in order of definition. | no | |

The `replace` array must contain hashes with the following keys:

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `placeholder` | string | The placeholder to be substituted by the command output. | yes | |
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
resources:
  - type: file
    parameters:
      path: /my/simple/example/file
```

```yaml
resources:
  - type: file
    parameters:
      ensure: present
      path: /my/simple/example/file
	  owner: myuser
	  group: mygroup
	  source: /some/child/path/in/the/asset/directory
```

```yaml
variables:
  sshd-port: 22
  sshd-allow-tcp-forwarding: yes

resources:
  - type: file
    parameters:
      ensure: present
      path: /etc/ssh/sshd_config
	  owner: root
	  group: root
	  content:
	    value: |
          Port ${pullconf::sshd-port}
		  Listen <ip>
		  
		  PermitRootLogin no
		  PasswordAuthentication no
		  PubkeyAuthentication yes
		  AllowTcpForwarding ${pullconf::sshd-allow-tcp-forwarding}
		replace:
		  - placeholder: <ip>
            command:
		      - /usr/local/sbin/determine-primary-ip-address.py
```
