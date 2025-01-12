# directory

This resource manages a directory within the filesystem hierarchy of the client.

## Relationship to other resources

The value of the `path` parameter must be unique among all directory, [file](file.md) and [symlink](symlink.md) resources.

A directory implicitly depends on other directory or [symlink](symlink.md) resources whose `path` parameters are ancestors to the directory `path`. For example when the `path` parameter of this directory is set to `/my/simple/example` and there is another directory whose `path` is `/my/simple`, then the former implicitly depends on the latter.

A directory also implicitly depends on [user](user.md) resources whose `home` parameter matches the directory's `path`. This is because the tools creating the user already create a home directory. So in most cases the home directory does not need to be managed by a directory resource. But when it is it will be applied after the user resource and operate on the existing home directory that was created by other means.

A directory also forms a relationship with child nodes, that is directory, [file](file.md) and [symlink](symlink.md) resources who this directory is a parent to. A directory resource needs to keep track of managed child nodes, because if will remove unmanaged child nodes if the `purge` parameter is set to `true`.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `path` | string | *Primary parameter*: An absolute filesystem path. | yes | |
| `owner` | string | The name of the user who owns the directory. | yes | `root` |
| `group` | string | The name of the group who owns the directory. | no | |
| `purge` | string | Determines if unknown child nodes should be removed from the directory. Unknown directories will be removed recursively as well. However note that when this directory contains another *managed* directory whose `purge` parameter is set to `false`, the contents of the other directory will remain intact and will not be recursively deleted. One of `true` or `false`. | no | `false` |

## Examples

```yaml
resources:
  - type: directory
    parameters:
      path: /my/simple/example
```

```yaml
resources:
  - type: directory
    parameters:
      ensure: present
      path: /my/simple/example
	  owner: myuser
	  group: mygroup
```

