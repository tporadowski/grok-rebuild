//! Subtree file counts stat every nested file. On a network mount or a
//! virtual filesystem that walk is the slow part, so `list_dir` skips it
//! and prints a notice instead of `[N files in subtree: …]`.
//!
//! overlayfs and tmpfs stay counted. Temp directories and this container's
//! root use them, and they are not the slow walks the note is about.

use std::path::Path;

/// Appended where a collapsed directory would have shown a subtree count.
pub(super) const SLOW_FS_SUBTREE_NOTICE: &str = "\nNote: subtree file counts were skipped \
because this directory is on a network or virtual filesystem.\n";

pub(super) fn subtree_counts_are_slow(path: &Path) -> bool {
    imp::subtree_counts_are_slow(path)
}

/// Linux `statfs(2)` `f_type`, low 32 bits. Network magics match
/// `xai_sqlite_journal::is_network_fs`. Virtual magics are kernel
/// filesystems whose trees are large or block on read. A failed probe
/// is not slow: an unclassifiable filesystem keeps the historical walk.
pub(super) fn slow_subtree_fs_magic(f_type: u64) -> bool {
    const NFS_SUPER_MAGIC: u64 = 0x6969;
    const SMB_SUPER_MAGIC: u64 = 0x517B;
    const SMB2_SUPER_MAGIC: u64 = 0xFE53_4D42;
    const CIFS_SUPER_MAGIC: u64 = 0xFF53_4D42;
    const V9FS_MAGIC: u64 = 0x0102_1997;
    const CODA_SUPER_MAGIC: u64 = 0x7375_7245;
    const AFS_SUPER_MAGIC: u64 = 0x5346_414F;
    const AFS_FS_MAGIC: u64 = 0x6B41_4653;
    const CEPH_SUPER_MAGIC: u64 = 0x00C3_6400;
    const LUSTRE_SUPER_MAGIC: u64 = 0x0BD0_0BD0;
    const GFS2_MAGIC: u64 = 0x0116_1970;
    const GPFS_SUPER_MAGIC: u64 = 0x4750_4653;
    const OCFS2_SUPER_MAGIC: u64 = 0x7461_636F;
    const WEKAFS_SUPER_MAGIC: u64 = 0x1803_1977;
    const FUSE_SUPER_MAGIC: u64 = 0x6573_5546;
    const PROC_SUPER_MAGIC: u64 = 0x9fa0;
    const SYSFS_MAGIC: u64 = 0x6265_6572;
    const DEBUGFS_MAGIC: u64 = 0x6462_6720;
    const TRACEFS_MAGIC: u64 = 0x7472_6163;
    const CGROUP_SUPER_MAGIC: u64 = 0x27e0eb;
    const CGROUP2_SUPER_MAGIC: u64 = 0x6367_7270;
    const SECURITYFS_MAGIC: u64 = 0x7363_6673;
    const BPF_FS_MAGIC: u64 = 0xcafe_4a11;
    const NSFS_MAGIC: u64 = 0x6e73_6673;
    const AUTOFS_SUPER_MAGIC: u64 = 0x0187;
    const DEVPTS_SUPER_MAGIC: u64 = 0x1cd1;
    const HUGETLBFS_MAGIC: u64 = 0x9584_58f6;
    const XENFS_SUPER_MAGIC: u64 = 0xabba_1974;
    const SELINUX_MAGIC: u64 = 0xf97c_ff8c;
    const SMACK_MAGIC: u64 = 0x4341_5d53;

    matches!(
        f_type & 0xFFFF_FFFF,
        NFS_SUPER_MAGIC
            | SMB_SUPER_MAGIC
            | SMB2_SUPER_MAGIC
            | CIFS_SUPER_MAGIC
            | V9FS_MAGIC
            | CODA_SUPER_MAGIC
            | AFS_SUPER_MAGIC
            | AFS_FS_MAGIC
            | CEPH_SUPER_MAGIC
            | LUSTRE_SUPER_MAGIC
            | GFS2_MAGIC
            | GPFS_SUPER_MAGIC
            | OCFS2_SUPER_MAGIC
            | WEKAFS_SUPER_MAGIC
            | FUSE_SUPER_MAGIC
            | PROC_SUPER_MAGIC
            | SYSFS_MAGIC
            | DEBUGFS_MAGIC
            | TRACEFS_MAGIC
            | CGROUP_SUPER_MAGIC
            | CGROUP2_SUPER_MAGIC
            | SECURITYFS_MAGIC
            | BPF_FS_MAGIC
            | NSFS_MAGIC
            | AUTOFS_SUPER_MAGIC
            | DEVPTS_SUPER_MAGIC
            | HUGETLBFS_MAGIC
            | XENFS_SUPER_MAGIC
            | SELINUX_MAGIC
            | SMACK_MAGIC
    )
}

/// macOS `statfs(2)` `f_fstypename`. Remote names match the journal
/// classifier. Virtual names are device, automount, and proc trees.
#[cfg(any(target_os = "macos", test))]
pub(super) fn slow_subtree_fs_name(fstype: &str) -> bool {
    fstype.eq_ignore_ascii_case("nfs")
        || fstype.eq_ignore_ascii_case("smbfs")
        || fstype.eq_ignore_ascii_case("cifs")
        || fstype.eq_ignore_ascii_case("afpfs")
        || fstype.eq_ignore_ascii_case("webdav")
        || fstype.eq_ignore_ascii_case("macfuse")
        || fstype.eq_ignore_ascii_case("osxfuse")
        || fstype.eq_ignore_ascii_case("fuse")
        || fstype.eq_ignore_ascii_case("devfs")
        || fstype.eq_ignore_ascii_case("autofs")
        || fstype.eq_ignore_ascii_case("procfs")
        || fstype.eq_ignore_ascii_case("linprocfs")
        || fstype.eq_ignore_ascii_case("synthfs")
        || fstype.eq_ignore_ascii_case("fdesc")
        || fstype.eq_ignore_ascii_case("volfs")
        || fstype.eq_ignore_ascii_case("virtiofs")
}

/// `\\server\share` and `\\?\UNC\server\share`. `\\.\` and `\\?\C:\` are local.
#[cfg(any(windows, test))]
pub(super) fn is_windows_unc(path: &str) -> bool {
    let Some(rest) = path.strip_prefix(r"\\") else {
        return false;
    };
    if let Some(verbatim) = rest.strip_prefix(r"?\") {
        return verbatim
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(r"UNC\"));
    }
    !rest.starts_with(r".\")
}

#[cfg(target_os = "linux")]
mod imp {
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub(super) fn subtree_counts_are_slow(path: &Path) -> bool {
        let Ok(cpath) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        // SAFETY: statfs is zero-initializable POD; cpath is NUL-terminated
        // and st is a valid out-pointer for the duration of the call.
        let mut st: libc::statfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statfs(cpath.as_ptr(), &mut st) } != 0 {
            return false;
        }
        super::slow_subtree_fs_magic(st.f_type as u64)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::path::Path;

    pub(super) fn subtree_counts_are_slow(path: &Path) -> bool {
        let Ok(stat) = nix::sys::statfs::statfs(path) else {
            return false;
        };
        // Absence of MNT_LOCAL is the remote signal, including fstypes we
        // do not name. Virtual filesystems can still be local.
        let remote = !stat.flags().contains(nix::mount::MntFlags::MNT_LOCAL);
        remote || super::slow_subtree_fs_name(stat.filesystem_type_name())
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    pub(super) fn subtree_counts_are_slow(path: &Path) -> bool {
        if super::is_windows_unc(&path.to_string_lossy()) {
            return true;
        }
        windows_mapped_drive_is_remote(path)
    }

    fn windows_mapped_drive_is_remote(path: &Path) -> bool {
        use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW};
        use windows::core::PCWSTR;
        // `DRIVE_REMOTE` from Win32. The named constant sits in
        // `Win32_System_WindowsProgramming`, which this crate does not enable.
        const DRIVE_REMOTE: u32 = 4;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut root = [0u16; 261];
        unsafe {
            if GetVolumePathNameW(PCWSTR(wide.as_ptr()), &mut root).is_err() {
                return false;
            }
            GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_REMOTE
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod imp {
    use std::path::Path;

    pub(super) fn subtree_counts_are_slow(_path: &Path) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_subtree_fs_magic_skips_network_and_virtual_not_local_disks() {
        for magic in [
            0x6969u64,   // NFS
            0x517B,      // SMB
            0xFE53_4D42, // SMB2
            0xFF53_4D42, // CIFS
            0x0102_1997, // 9p
            0x7375_7245, // CODA
            0x5346_414F, // AFS
            0x6B41_4653, // kAFS
            0x00C3_6400, // CEPH
            0x0BD0_0BD0, // Lustre
            0x0116_1970, // GFS2
            0x4750_4653, // GPFS
            0x7461_636F, // OCFS2
            0x1803_1977, // wekafs
            0x6573_5546, // FUSE
            0x9fa0,      // proc
            0x6265_6572, // sysfs
            0x6462_6720, // debugfs
            0x7472_6163, // tracefs
            0x27e0eb,    // cgroup
            0x6367_7270, // cgroup2
            0x7363_6673, // securityfs
            0xcafe_4a11, // bpf
            0x6e73_6673, // nsfs
            0x0187,      // autofs
            0x1cd1,      // devpts
        ] {
            assert!(slow_subtree_fs_magic(magic), "magic {magic:#x}");
        }
        // A 32-bit kernel sign-extends CIFS's high bit.
        assert!(slow_subtree_fs_magic(0xFFFF_FFFF_FF53_4D42));
        for magic in [
            0xEF53u64,   // ext2/3/4
            0x0102_1994, // tmpfs
            0x794C_7630, // overlayfs
            0x9123_683E, // btrfs
            0x5846_5342, // XFS
            0x8584_58f6, // ramfs
            0x0,
        ] {
            assert!(!slow_subtree_fs_magic(magic), "magic {magic:#x}");
        }
    }

    #[test]
    fn slow_subtree_fs_name_matches_network_and_virtual() {
        for name in [
            "nfs", "smbfs", "cifs", "afpfs", "webdav", "NFS", "macfuse", "osxfuse", "fuse",
            "devfs", "autofs", "procfs", "linprocfs", "synthfs", "fdesc", "volfs", "virtiofs",
        ] {
            assert!(slow_subtree_fs_name(name), "{name}");
        }
        for name in ["apfs", "hfs", "tmpfs", "ext4", "overlay", ""] {
            assert!(!slow_subtree_fs_name(name), "{name}");
        }
    }

    #[test]
    fn windows_unc_is_a_slow_subtree_filesystem() {
        assert!(is_windows_unc(r"\\server\share"));
        assert!(is_windows_unc(r"\\?\UNC\server\share"));
        assert!(is_windows_unc(r"\\?\unc\server\share"));
        assert!(!is_windows_unc(r"\\.\pipe\name"));
        assert!(!is_windows_unc(r"\\?\C:\Users"));
        assert!(!is_windows_unc(r"C:\Users"));
        assert!(!is_windows_unc("/proc/self"));
    }
}
