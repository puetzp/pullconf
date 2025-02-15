# Triggers

Triggers are closely tied to the [execute](resources/execute.md) resource. Every resource has the ability to trigger an [execute](resources/execute.md) resource through the `triggers` meta-parameter. Just like `requires` this meta-parameter is available to all resources by default.

When the `passive` parameter of an [execute](resources/execute.md) resource is set to `true`, the resource only applies when triggered by other resources. By default a resource triggers an `execute` resource when the following conditions apply:

* the triggering resource is successfully `created`
* the triggering resource is successfully `deleted`
* the triggering resource is successfully `changed`

These conditions can be altered through the `when` parameter. This enables use cases where some `execute` resource should only be triggered when a resource is `created` and another should be triggered when the same resource is `deleted`.

When the `when` parameter is omitted, the default behavior above applies.

When multiple resources are configured to trigger the same [execute](resources/execute.md) resource, the latter is applied only once, after each of those resources have run to completion.

Since only [execute](resources/execute.md) resources can be triggered, the `triggers` meta-parameter accepts only references to [execute](resources/execute.md) resources.

## Example

```yaml
<...>

resources:
  - type: execute
    parameters:
	  ensure: present
      name: reload-sshd
	  command:
	    - systemctl
	    - reload
		- sshd.service
	  passive: true

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
        # Reload sshd only when the file is created or changed.
		when:
		  - created
		  - changed

<...>
```

```yaml
<...>

resources:
  - type: execute
    parameters:
	  ensure: present
      name: daemon-reload
	  command:
	    - systemctl
	    - daemon-reload
	  passive: true

  - type: file
    parameters:
      ensure: present
      path: /etc/systemd/system/some-unit.service
	  owner: root
	  group: root
	  content:
	    value: |
          ...
    triggers:
      - type: execute
        name: daemon-reload
		# Thie is the default behavior and the same as if `when` is omitted altogether.
		when:
		  - created
		  - changed
		  - deleted

<...>
```
