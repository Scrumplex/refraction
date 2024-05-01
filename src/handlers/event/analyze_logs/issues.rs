use crate::{api, utils::semver_split, Data};

use std::sync::OnceLock;

use eyre::Result;
use log::trace;
use regex::Regex;

pub type Issue = Option<(String, String)>;

pub async fn find(log: &str, data: &Data) -> Result<Vec<(String, String)>> {
	trace!("Checking log for issues");

	let issues = [
		fabric_internal,
		flatpak_nvidia,
		forge_java,
		intel_hd,
		java_option,
		lwjgl_2_java_9,
		macos_ns,
		oom,
		optinotfine,
		pre_1_12_native_transport_java_9,
		wrong_java,
	];

	let mut res: Vec<(String, String)> = issues.iter().filter_map(|issue| issue(log)).collect();

	if let Some(issues) = outdated_launcher(log, data).await? {
		res.push(issues);
	}

	Ok(res)
}

fn fabric_internal(log: &str) -> Issue {
	const CLASS_NOT_FOUND: &str = "Caused by: java.lang.ClassNotFoundException: ";

	let issue = (
		"Fabric Internal Access".to_string(),
		"The mod you are using is using fabric internals that are not meant \
        to be used by anything but the loader itself.
        Those mods break both on Quilt and with fabric updates.
        If you're using fabric, downgrade your fabric loader could work, \
        on Quilt you can try updating to the latest beta version, \
        but there's nothing much to do unless the mod author stops using them."
			.to_string(),
	);

	let errors = [
		&format!("{CLASS_NOT_FOUND}net.fabricmc.fabric.impl"),
		&format!("{CLASS_NOT_FOUND}net.fabricmc.fabric.mixin"),
		&format!("{CLASS_NOT_FOUND}net.fabricmc.fabric.loader.impl"),
		&format!("{CLASS_NOT_FOUND}net.fabricmc.fabric.loader.mixin"),
		"org.quiltmc.loader.impl.FormattedException: java.lang.NoSuchMethodError:",
	];

	let found = errors.iter().any(|e| log.contains(e));
	found.then_some(issue)
}

fn flatpak_nvidia(log: &str) -> Issue {
	let issue = (
		"Outdated Nvidia Flatpak Driver".to_string(),
		"The Nvidia driver for flatpak is outdated.
        Please run `flatpak update` to fix this issue. \
        If that does not solve it, \
        please wait until the driver is added to Flathub and run it again."
			.to_string(),
	);

	let found = log.contains("org.lwjgl.LWJGLException: Could not choose GLX13 config")
		|| log.contains("GLFW error 65545: GLX: Failed to find a suitable GLXFBConfig");

	found.then_some(issue)
}

fn forge_java(log: &str) -> Issue {
	let issue = (
		"Forge Java Bug".to_string(),
		"Old versions of Forge crash with Java 8u321+.
            To fix this, update forge to the latest version via the Versions tab
            (right click on Forge, click Change Version, and choose the latest one)
            Alternatively, you can download 8u312 or lower. \
            See [archive](https://github.com/adoptium/temurin8-binaries/releases/tag/jdk8u312-b07)"
			.to_string(),
	);

	let found = log.contains("java.lang.NoSuchMethodError: sun.security.util.ManifestEntryVerifier.<init>(Ljava/util/jar/Manifest;)V");
	found.then_some(issue)
}

fn intel_hd(log: &str) -> Issue {
	let issue =
        (
        "Intel HD Windows 10".to_string(),
        "Your drivers don't support windows 10 officially
        See https://prismlauncher.org/wiki/getting-started/installing-java/#a-note-about-intel-hd-20003000-on-windows-10 for more info".to_string()
    );

	let found = log.contains("org.lwjgl.LWJGLException: Pixel format not accelerated");
	found.then_some(issue)
}

fn java_option(log: &str) -> Issue {
	static VM_OPTION_REGEX: OnceLock<Regex> = OnceLock::new();
	static UNRECOGNIZED_OPTION_REGEX: OnceLock<Regex> = OnceLock::new();

	let vm_option =
		VM_OPTION_REGEX.get_or_init(|| Regex::new(r"Unrecognized VM option '(.+)'[\r\n]").unwrap());
	let unrecognized_option = UNRECOGNIZED_OPTION_REGEX
		.get_or_init(|| Regex::new(r"Unrecognized option: (.+)[\r\n]").unwrap());

	if let Some(captures) = vm_option.captures(log) {
		let title = if &captures[1] == "UseShenandoahGC" {
			"Wrong Java Arguments"
		} else {
			"Java 8 and below don't support ShenandoahGC"
		};
		return Some((
			title.to_string(),
			format!("Remove `-XX:{}` from your Java arguments", &captures[1]),
		));
	}

	if let Some(captures) = unrecognized_option.captures(log) {
		return Some((
			"Wrong Java Arguments".to_string(),
			format!("Remove `{}` from your Java arguments", &captures[1]),
		));
	}

	None
}

fn lwjgl_2_java_9(log: &str) -> Issue {
	let issue = (
		"Linux: crash with pre-1.13 and Java 9+".to_string(),
		"Using pre-1.13 (which uses LWJGL 2) with Java 9 or later usually causes a crash. \
        Switching to Java 8 or below will fix your issue.
        Alternatively, you can use [Temurin](https://adoptium.net/temurin/releases). \
        However, multiplayer will not work in versions from 1.8 to 1.11.
        For more information, type `/tag java`."
			.to_string(),
	);

	let found = log.contains("check_match: Assertion `version->filename == NULL || ! _dl_name_match_p (version->filename, map)' failed!");
	found.then_some(issue)
}

fn macos_ns(log: &str) -> Issue {
	let issue = (
    "MacOS NSInternalInconsistencyException".to_string(),
    "You need to downgrade your Java 8 version. See https://prismlauncher.org/wiki/getting-started/installing-java/#older-minecraft-on-macos".to_string()
);

	let found =
		log.contains("Terminating app due to uncaught exception 'NSInternalInconsistencyException");
	found.then_some(issue)
}

fn oom(log: &str) -> Issue {
	let issue = (
		"Out of Memory".to_string(),
		"Allocating more RAM to your instance could help prevent this crash.".to_string(),
	);

	let found = log.contains("java.lang.OutOfMemoryError") || log.contains("-805306369");
	found.then_some(issue)
}

fn optinotfine(log: &str) -> Issue {
	let issue = (
        "Potential OptiFine Incompatibilities".to_string(),
        "OptiFine is known to cause problems when paired with other mods. \
        Try to disable OptiFine and see if the issue persists.
        Check `/tag optifine` for more info & some typically more compatible alternatives you can use."
            .to_string(),
    );

	let found = log.contains("[✔] OptiFine_") || log.contains("[✔] optifabric-");
	found.then_some(issue)
}

async fn outdated_launcher(log: &str, data: &Data) -> Result<Issue> {
	static OUTDATED_LAUNCHER_REGEX: OnceLock<Regex> = OnceLock::new();
	let outdated_launcher = OUTDATED_LAUNCHER_REGEX.get_or_init(|| {
		Regex::new("Prism Launcher version: ((?:([0-9]+)\\.)?([0-9]+)\\.([0-9]+))").unwrap()
	});

	let Some(captures) = outdated_launcher.captures(log) else {
		return Ok(None);
	};

	let octocrab = &data.octocrab;
	let log_version = &captures[1];
	let log_version_parts = semver_split(log_version);

	let latest_version = if let Some(storage) = &data.storage {
		if let Ok(version) = storage.launcher_version().await {
			version
		} else {
			let version = api::github::get_latest_prism_version(octocrab).await?;
			storage.cache_launcher_version(&version).await?;
			version
		}
	} else {
		trace!("Not caching launcher version, as we're running without a storage backend");
		api::github::get_latest_prism_version(octocrab).await?
	};
	let latest_version_parts = semver_split(&latest_version);

	if log_version_parts.len() != 2
		|| log_version_parts[0] < latest_version_parts[0]
		|| (log_version_parts[0] == latest_version_parts[0]
			&& log_version_parts[1] < latest_version_parts[1])
	{
		let issue = (
        	"Outdated Prism Launcher".to_string(),
        	format!("Your installed version is {log_version}, while the newest version is {latest_version}.\nPlease update; for more info see https://prismlauncher.org/download/")
        );

		Ok(Some(issue))
	} else {
		Ok(None)
	}
}

fn pre_1_12_native_transport_java_9(log: &str) -> Issue {
	let issue = (
        "Linux: broken multiplayer with 1.8-1.11 and Java 9+".to_string(),
        "These versions of Minecraft use an outdated version of Netty which does not properly support Java 9.

Switching to Java 8 or below will fix this issue. For more information, type `/tag java`.

If you must use a newer version, do the following:
- Open `options.txt` (in the main window Edit -> Open .minecraft) and change.
- Find `useNativeTransport:true` and change it to `useNativeTransport:false`.
Note: whilst Netty was introduced in 1.7, this option did not exist \
which is why the issue was not present."
            .to_string(),
    );

	let found = log.contains(
        "java.lang.RuntimeException: Unable to access address of buffer\n\tat io.netty.channel.epoll"
    );

	found.then_some(issue)
}

fn wrong_java(log: &str) -> Issue {
	static SWITCH_VERSION_REGEX: OnceLock<Regex> = OnceLock::new();
	let switch_version = SWITCH_VERSION_REGEX.get_or_init(|| Regex::new(
		r"(?m)Please switch to one of the following Java versions for this instance:[\r\n]+(Java version [\d.]+)",
).unwrap());

	if let Some(captures) = switch_version.captures(log) {
		let versions = captures[1].split('\n').collect::<Vec<&str>>().join(", ");
		return Some((
            "Wrong Java Version".to_string(),
            format!("Please switch to one of the following: `{versions}`\nFor more information, type `/tag java`"),
        ));
	}

	let issue = (
        "Java compatibility check skipped".to_string(),
        "The Java major version may not work with your Minecraft instance. Please switch to a compatible version".to_string()
    );

	log.contains("Java major version is incompatible. Things might break.")
		.then_some(issue)
}
