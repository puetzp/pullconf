# Resources

Resources are the main building blocks of your configuration. They define which [files](file.md), [directories](directory.md) etc. are created and managed on the client.

Resources can be defined inside the `resources` array in [clients](../client.md) and [groups](../group.md).

Every resource is structured the same at the root and contains the following keys:

| Name | Description | Mandatory | Default |
| --- | --- | --- | --- |
| `type` | A string that determines how to parse this resource. Must match one of the supported resource type identifiers. See below for valid values. | yes | |
| `parameters` | A hash of resource-specific parameters. | yes | |
| `requires` | An array of dependencies in the form of references to other resources. See [dependencies](../dependencies.md). | no | `[]` |
| `triggers` | An array of triggers in the form of references to `execute` resources. See [triggers](../triggers.md). | no | `[]` |

Valid values for `type` are:

- `apt::package`
- `directory`
- `execute`
- `file`
- `group`
- `host`
- `symlink`
- `user`

The keys inside `parameters` differ between resources, so the respective resource documentation section should be consulted to see the available resource parameters.

Also note that [variables](../variables.md) can only be used to substitute values inside the `parameters` hash. Root-level keys such as `type` cannot be substituted.

## Example

```yaml
<...>

resources:
  - type: directory
    parameters:
	  ensure: present
	  path: /path/to/directory
	  purge: true

  - type: file
    parameters:
	  ensure: present
	  path: /path/to/directory/somefile

  - type: execute
    parameters:
	  name: reload
	  command:
	    - systemctl
		- daemon-reload
	  passive: true

  - type: file
    parameters:
	  ensure: present
	  path: /path/to/directory/otherfile
    requires:
	  - type: file
	    path: /path/to/directory/somefile
	triggers:
	  - type: execute
	    name: reload
```

```yaml
<...>

# All kinds of dependencies.
resources:
  - type: directory
    parameters:
	  ensure: present
	  path: /path/to/directory
    requires:
	  - type: apt::package
	    name: some-package

      - type: directory
	    path: /some/directory

      - type: execute
	    name: my-script

      - type: file
	    path: /some/file
	  
	  - type: group
	    name: some-group
	  
	  - type: host
	    ip_address: 172.16.0.1
	  
	  - type: symlink
	    path: /some/symlink
	  
	  - type: user
	    name: some-user
```
