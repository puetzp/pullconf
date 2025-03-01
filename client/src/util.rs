use anyhow::Context;
use common::{resources::group::Name as Groupname, resources::user::Name as Username};
use log::debug;
use nix::unistd::{Group, User};

/// Query the system for a user and optionally a group by their respective names
/// and return their numeric IDs.
/// Default to the user's primary group if no group name has been provided for
/// the search.
/// Fail if either the user or group cannot be found and thus apparently do not
/// exist in the system.
pub fn uid_and_gid(
    owner: &Username,
    group: &Option<Groupname>,
) -> Result<(u32, u32), anyhow::Error> {
    debug!("querying system for user and group data");

    let error = format!("failed to find user '{}'", owner);

    let user = match User::from_name(owner).context(error.clone())? {
        Some(user) => user,
        None => anyhow::bail!(error),
    };

    let gid = match group {
        Some(name) => {
            let error = format!("failed to find group '{}'", name);

            match Group::from_name(name).context(error.clone())? {
                Some(group) => u32::from(group.gid),
                None => anyhow::bail!(error),
            }
        }
        None => u32::from(user.gid),
    };

    let uid = u32::from(user.uid);

    Ok((uid, gid))
}
