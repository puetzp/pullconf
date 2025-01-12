# apt::preference

This resource manages a file containing a single preference record in the directory `/etc/apt/preferences.d`. Files in this directory are taken into consideration by `apt` when it assigns priorities to different versions of packages to determine which is selected for installation. Files in this directory are parsed in alphanumeric ascending order.

For more information about apt preferences see `man apt_preferences`.

## Relationship to other resources

This resource implicitly depends on [directory](../directory.md) and [symlink](../symlink.md) resources whose `path` parameters are ancestors to the directory `/etc/apt/preferences.d`.

## Parameters

| Name | Type | Description | Mandatory | Default |
| --- | --- | --- | --- | --- |
| `ensure` | string | Determines the desired state of the resource. One of `present` or `absent`. | yes | `present` |
| `name` | string | *Primary parameter*: Name of the preference. The name will also be used as file name. | yes | |
| `order` | string | A number that will be prepended to the file name to control the place in which `apt` parses the file. Files of lower oder will be parsed earlier. | no | |
| `explanation` | string | The value of the `Explanation` field as described in the man pages. | no | |
| `package` | string | The value of the `Package` field as described in the man pages. | yes | |
| `pin` | string | The value of the `Pin` field as described in the man pages. | yes | |
| `pin_priority` | string | The value of the `Pin-Priority` field as described in the man pages. | yes | |

## Examples

```yaml
resources:
  - type: apt::preference
    parameters:
	  name: nginx
	  explanation: Pin nginx at this version even if it results in a downgrade.
	  package: nginx
	  pin: version 1.18.0*
	  pin_priority: 1000
```

```yaml
resources:
  - type: apt::preference
    parameters:
      ensure: present
	  name: bookworm
	  order: 10
	  explanation: Assign a higher priority to packages from a distribution by the codename bookworm.
      package: *
      pin: release n=bookworm
      pin_priority: 900
```

