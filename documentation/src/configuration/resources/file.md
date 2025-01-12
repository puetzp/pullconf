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
| `content` | string | The content to be written as-is to the file. Mutually exclusive with `source`. | no | |
| `source` | string | An absolute path to a file asset on the server. The remote file will be downloaded and its contents written to the file on the client. For example when this parameter is set to `/my/example/file` the file must exist on the server at `$PULLCONF_ASSET_DIR/my/example/file` in order to be downloaded successfully by the client. Mutually exclusive with `content`. | no | |

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
resources:
  - type: file
    parameters:
      ensure: present
      path: /my/simple/example/file
	  owner: myuser
	  group: mygroup
	  content: |
	    my
		multiline
		example
		file content
```
