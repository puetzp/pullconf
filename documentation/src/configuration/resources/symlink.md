# symlink

This resource manages a symbolic link within the filesystem hierarchy of the client.

## Relationship to other resources

The value of the `path` parameter must be unique among all symlink, [file](file.md) and [directory](directory.md) resources.

A symlink implicitly depends on other symlinks or [directory](directory.md) resources whose `path` parameters are ancestors to this symlink's `path`. For example when the `path` parameter of this symlink is set to `/my/very/simple/example` and there is a directory resource whose `path` is `/my/very/simple`, then the former implicitly depends on the latter.

If the `target` parameter of the symlink resource contains a path that matches the `path` parameters of a managed [file](file.md) or [directory](directory.md) resource, the symlink resource depends on the latter.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `path` | string | *Primary parameter*: An absolute filesystem path. | yes | |
| `target` | string | An absolute filesystem path that the symlink should point to. | yes | |

## Examples

```yaml
resources:
  - type: symlink
    parameters:
      path: /my/simple/example/link
	  target: /my/simple/target
```

