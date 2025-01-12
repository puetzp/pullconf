# apt::package

This resource manages a package via `apt`.

## Relationship to other resources

At this point apt::package resources do not form implicit dependencies with other types of resources.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present`, `absent` or `purged`[^note]. | yes | `present` |
| `name` | string | *Primary parameter*: Name of the package. | yes | |
| `version` | string | The version to be installed. Stating a specific version might result in a *downgrade*. This parameter can be omitted to install the latest known version instead. | no | |

[^note]: `purged` means that in addition to the package being deleted (just like `absent`), all configuration files and directories associated with the package are deleted as well.

## Examples

```yaml
resources:
  - type: apt::package
    parameters:
	  name: nginx
```

```yaml
resources:
  - type: apt::package
    parameters:
	  ensure: present
	  name: nginx
	  version: 1.18.0-6ubuntu14.4
```

