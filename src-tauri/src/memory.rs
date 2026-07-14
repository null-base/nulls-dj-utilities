use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

#[derive(Default)]
pub(crate) struct LiveTrack {
    pub(crate) file_path: String,
    pub(crate) title: String,
    pub(crate) artist: String,
    pub(crate) album: String,
    pub(crate) bpm: Option<f32>,
    pub(crate) position_seconds: Option<f64>,
    pub(crate) duration_seconds: Option<f64>,
    pub(crate) source: String,
    pub(crate) master_verified: bool,
}

fn parse_track_info(text: &str) -> Option<LiveTrack> {
    let mut track = LiveTrack::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':').or_else(|| line.split_once('：')) else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "Title" | "Track Title" | "トラックタイトル" | "タイトル" => {
                track.title = value
            }
            "Artist" | "アーティスト" => track.artist = value,
            "Album" | "アルバム" => track.album = value,
            _ => {}
        }
    }
    (!track.title.is_empty()).then_some(track)
}

fn track_text_from_bytes(bytes: &[u8]) -> Option<String> {
    let utf8 = bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .collect::<Vec<_>>();
    if let Ok(text) = String::from_utf8(utf8) {
        if text.contains("Title")
            || text.contains("Artist")
            || text.contains("Album")
            || text.contains("トラックタイトル")
            || text.contains("アーティスト")
            || text.contains("アルバム")
        {
            return Some(text);
        }
    }

    let units = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .take_while(|unit| *unit != 0)
        .collect::<Vec<_>>();
    String::from_utf16(&units).ok()
}

fn debug_memory_text(
    platform: &str,
    context: impl AsRef<str>,
    deck: usize,
    address: usize,
    text: &str,
) {
    debug_memory_log(format!(
        "[null's DJ Utilities][rekordbox-memory][{platform}] {} deck={} addr=0x{address:X} text=\"{}\"",
        context.as_ref(),
        deck + 1,
        sanitize_debug_text(text)
    ));
}

fn debug_memory_status(platform: &str, message: impl AsRef<str>) {
    debug_memory_log(format!(
        "[null's DJ Utilities][rekordbox-memory][{platform}] {}",
        message.as_ref()
    ));
}

fn debug_memory_log(message: impl AsRef<str>) {
    use std::io::Write as _;

    let message = message.as_ref();
    eprintln!("{message}");

    let path = std::env::temp_dir().join("nulls-dj-utilities-rekordbox-memory.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
    }
}

fn sanitize_debug_text(text: &str) -> String {
    let mut text = text
        .replace('\0', "\\0")
        .lines()
        .take(12)
        .collect::<Vec<_>>()
        .join(" | ");
    if text.chars().count() > 1000 {
        text = text.chars().take(1000).collect::<String>();
        text.push_str("...");
    }
    text
}

const TRACK_INFO_BYTES: usize = 2000;
const MAX_ART_REGION_BYTES: usize = 8 * 1024 * 1024;
const MAX_ART_BYTES: usize = 2 * 1024 * 1024;
const MIN_ART_BYTES: usize = 32;
const MAX_ART_SCAN_REGIONS: usize = 160;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const JPEG_SIGNATURE: &[u8] = b"\xff\xd8";

fn album_art_data_url_from_bytes(bytes: &[u8]) -> Option<(usize, String)> {
    let mut best: Option<(&str, &[u8])> = None;
    for (mime, image) in png_images(bytes).chain(jpeg_images(bytes)) {
        if image.len() < MIN_ART_BYTES || image.len() > MAX_ART_BYTES {
            continue;
        }
        if best.is_none_or(|(_, best)| image.len() > best.len()) {
            best = Some((mime, image));
        }
    }
    best.map(|(mime, image)| {
        (
            image.len(),
            format!("data:{mime};base64,{}", BASE64.encode(image)),
        )
    })
}

fn png_images(bytes: &[u8]) -> impl Iterator<Item = (&'static str, &[u8])> {
    find_all(bytes, PNG_SIGNATURE).filter_map(|start| {
        let end = png_end(bytes, start)?;
        Some(("image/png", &bytes[start..end]))
    })
}

fn png_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start.checked_add(PNG_SIGNATURE.len())?;
    while cursor.checked_add(12)? <= bytes.len() {
        let length = u32::from_be_bytes(bytes.get(cursor..cursor + 4)?.try_into().ok()?) as usize;
        let chunk_type = bytes.get(cursor + 4..cursor + 8)?;
        let next = cursor.checked_add(12)?.checked_add(length)?;
        if next.checked_sub(start)? > MAX_ART_BYTES || next > bytes.len() {
            return None;
        }
        if chunk_type == b"IEND" {
            return Some(next);
        }
        cursor = next;
    }
    None
}

fn jpeg_images(bytes: &[u8]) -> impl Iterator<Item = (&'static str, &[u8])> {
    find_all(bytes, JPEG_SIGNATURE).filter_map(|start| {
        let search_from = start.checked_add(2)?;
        let relative_end = find_all(bytes.get(search_from..)?, b"\xff\xd9").next()?;
        let end = search_from.checked_add(relative_end)?.checked_add(2)?;
        Some(("image/jpeg", &bytes[start..end]))
    })
}

fn find_all<'a>(bytes: &'a [u8], needle: &'a [u8]) -> impl Iterator<Item = usize> + 'a {
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(move |(index, window)| (window == needle).then_some(index))
}

fn best_album_art_from_regions(
    platform: &str,
    regions: impl IntoIterator<Item = (usize, usize)>,
    mut read: impl FnMut(usize, usize) -> Option<Vec<u8>>,
) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    let mut scanned = 0;
    for (base, size) in regions {
        if size == 0 || size > MAX_ART_REGION_BYTES {
            continue;
        }
        scanned += 1;
        if scanned > MAX_ART_SCAN_REGIONS {
            break;
        }
        let Some(bytes) = read(base, size) else {
            continue;
        };
        let Some(candidate) = album_art_data_url_from_bytes(&bytes) else {
            continue;
        };
        if best.as_ref().is_none_or(|(len, _)| candidate.0 > *len) {
            best = Some(candidate);
        }
    }
    if let Some((len, url)) = best {
        debug_memory_status(platform, format!("album art found in memory bytes={len}"));
        Some(url)
    } else {
        debug_memory_status(
            platform,
            format!("album art not found in memory scanned_regions={scanned}"),
        );
        None
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::{collections::HashMap, ffi::OsString, mem, os::windows::ffi::OsStringExt};

    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::{
                Debug::ReadProcessMemory,
                ToolHelp::{
                    CreateToolhelp32Snapshot, Module32FirstW, Process32FirstW, Process32NextW,
                    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
                    TH32CS_SNAPPROCESS,
                },
            },
            Memory::{
                VirtualQueryEx, MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READ,
                PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_NOACCESS,
                PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY,
            },
            Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
        },
    };

    const AUDIO_EXTENSIONS: &[&str] = &[
        ".mp3", ".wav", ".flac", ".aiff", ".aif", ".m4a", ".aac", ".ogg",
    ];
    const MAX_REGION: usize = 16 * 1024 * 1024;
    const RKBX_LINK_OFFSETS: &str = include_str!("rkbx_link_offsets.txt");

    pub(crate) fn current_track_path() -> Result<String, String> {
        let pid = find_rekordbox_pid().ok_or_else(|| "rekordbox.exe was not found".to_string())?;
        let process = ProcessHandle::open(pid)?;
        scan_memory_for_audio_path(process.0)
    }

    pub(crate) fn current_track() -> Result<super::LiveTrack, String> {
        let Some(pid) = find_rekordbox_pid() else {
            super::debug_memory_status("windows", "rekordbox.exe was not found");
            return Err("rekordbox.exe was not found".to_string());
        };
        let process = ProcessHandle::open(pid)?;
        if let Some(track) = read_master_track_from_offsets(process.0, pid) {
            return Ok(track);
        }
        super::debug_memory_status(
            "windows",
            "offset metadata read failed; falling back to audio path scan",
        );
        scan_memory_for_audio_path(process.0).map(|file_path| super::LiveTrack {
            file_path,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            bpm: None,
            position_seconds: None,
            duration_seconds: None,
            source: "Rekordbox".to_string(),
            master_verified: false,
        })
    }

    pub(crate) fn current_album_art_data_url() -> Option<String> {
        let pid = find_rekordbox_pid()?;
        let process = ProcessHandle::open(pid).ok()?;
        super::best_album_art_from_regions("windows", readable_regions(process.0), |base, size| {
            read_memory(process.0, base, size)
        })
    }

    fn scan_memory_for_audio_path(handle: HANDLE) -> Result<String, String> {
        let regions = readable_regions(handle);
        if regions.is_empty() {
            return Err("no readable rekordbox memory regions".to_string());
        }

        let mut scores: HashMap<String, usize> = HashMap::new();
        for (base, size) in regions {
            if size == 0 || size > MAX_REGION {
                continue;
            }
            let Some(bytes) = read_memory(handle, base, size) else {
                continue;
            };
            for path in extract_ascii_paths(&bytes) {
                *scores.entry(path).or_default() += 1;
            }
            for path in extract_utf16_paths(&bytes) {
                *scores.entry(path).or_default() += 1;
            }
        }

        scores
            .into_iter()
            .max_by_key(|(path, count)| (*count, path.len()))
            .map(|(path, _)| path)
            .ok_or_else(|| "no audio path found in rekordbox memory".to_string())
    }

    #[derive(Clone)]
    struct Pointer {
        offsets: Vec<usize>,
        final_offset: usize,
    }

    struct OffsetSet {
        version: String,
        masterdeck_index: Pointer,
        current_bpm: Vec<Pointer>,
        sample_position: Vec<Pointer>,
        track_info: Vec<Pointer>,
    }

    fn read_master_track_from_offsets(handle: HANDLE, pid: u32) -> Option<super::LiveTrack> {
        let base = rekordbox_module_base_for_pid(pid)?;
        let offset_sets = parse_offsets(RKBX_LINK_OFFSETS);
        super::debug_memory_status(
            "windows",
            format!(
                "pid={pid} base=0x{base:X} offset_sets={}",
                offset_sets.len()
            ),
        );
        for (set_index, offsets) in offset_sets.iter().enumerate() {
            let Some(master_addr) = resolve_pointer(handle, base, &offsets.masterdeck_index) else {
                super::debug_memory_status(
                    "windows",
                    format!("set={} master deck pointer failed", set_index + 1),
                );
                continue;
            };
            let Some(deck) = read_u8(handle, master_addr).map(usize::from) else {
                super::debug_memory_status(
                    "windows",
                    format!(
                        "set={} master deck read failed addr=0x{master_addr:X}",
                        set_index + 1
                    ),
                );
                continue;
            };
            let Some(pointer) = offsets.track_info.get(deck) else {
                super::debug_memory_status(
                    "windows",
                    format!(
                        "set={} deck={} has no track pointer",
                        set_index + 1,
                        deck + 1
                    ),
                );
                continue;
            };
            let Some(track_addr) = resolve_pointer(handle, base, pointer) else {
                super::debug_memory_status(
                    "windows",
                    format!(
                        "set={} deck={} track pointer failed",
                        set_index + 1,
                        deck + 1
                    ),
                );
                continue;
            };
            let Some(text) = read_track_text(handle, track_addr, super::TRACK_INFO_BYTES) else {
                super::debug_memory_status(
                    "windows",
                    format!(
                        "set={} deck={} text decode failed addr=0x{track_addr:X}",
                        set_index + 1,
                        deck + 1
                    ),
                );
                continue;
            };
            super::debug_memory_text(
                "windows",
                format!("set={} version={}", set_index + 1, offsets.version),
                deck,
                track_addr,
                &text,
            );
            let Some(track) = super::parse_track_info(&text) else {
                super::debug_memory_status(
                    "windows",
                    format!(
                        "set={} deck={} decoded text has no title label",
                        set_index + 1,
                        deck + 1
                    ),
                );
                continue;
            };
            let track = super::LiveTrack {
                bpm: offsets
                    .current_bpm
                    .get(deck)
                    .and_then(|pointer| read_pointer_value::<4>(handle, base, pointer))
                    .map(f32::from_ne_bytes)
                    .filter(|value| value.is_finite() && *value > 0.0),
                // ponytail: known Rekordbox offsets expose sample position; add per-track sample rate if needed.
                position_seconds: offsets
                    .sample_position
                    .get(deck)
                    .and_then(|pointer| read_pointer_value::<8>(handle, base, pointer))
                    .map(i64::from_ne_bytes)
                    .filter(|value| *value > 0)
                    .map(|value| value as f64 / 44_100.0),
                source: "Rekordbox".to_string(),
                master_verified: true,
                ..track
            };
            if !track.title.is_empty() {
                return Some(track);
            }
        }
        None
    }

    fn parse_offsets(input: &str) -> Vec<OffsetSet> {
        let lines = input
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>();
        if lines.len() < 6 {
            return Vec::new();
        }

        let mut sets = Vec::new();
        let mut index = 0;
        while index + 1 < lines.len() {
            let version = lines[index].to_string();
            let Some(masterdeck_index) = parse_pointer(lines[index + 1]) else {
                break;
            };
            index += 2;

            let mut track_info = Vec::new();
            let mut current_bpm = Vec::new();
            let mut sample_position = Vec::new();
            while index + 3 < lines.len() && parse_pointer(lines[index]).is_some() {
                if let Some(pointer) = parse_pointer(lines[index]) {
                    current_bpm.push(pointer);
                }
                if let Some(pointer) = parse_pointer(lines[index + 1]) {
                    sample_position.push(pointer);
                }
                if let Some(pointer) = parse_pointer(lines[index + 2]) {
                    track_info.push(pointer);
                }
                index += 4;
            }

            if !track_info.is_empty() {
                sets.push(OffsetSet {
                    version,
                    masterdeck_index,
                    current_bpm,
                    sample_position,
                    track_info,
                });
            }
        }
        sets
    }

    fn parse_pointer(line: &str) -> Option<Pointer> {
        let values = line
            .split_whitespace()
            .map(|part| usize::from_str_radix(part, 16).ok())
            .collect::<Option<Vec<_>>>()?;
        let (final_offset, offsets) = values.split_last()?;
        Some(Pointer {
            offsets: offsets.to_vec(),
            final_offset: *final_offset,
        })
    }

    fn resolve_pointer(handle: HANDLE, base: usize, pointer: &Pointer) -> Option<usize> {
        let mut address = base;
        for offset in &pointer.offsets {
            address = read_usize(handle, address.checked_add(*offset)?)?;
            if address < 0x10000 {
                return None;
            }
        }
        address.checked_add(pointer.final_offset)
    }

    fn read_pointer_value<const N: usize>(
        handle: HANDLE,
        base: usize,
        pointer: &Pointer,
    ) -> Option<[u8; N]> {
        let address = resolve_pointer(handle, base, pointer)?;
        let bytes = read_memory(handle, address, N)?;
        bytes.get(..N)?.try_into().ok()
    }

    fn read_usize(handle: HANDLE, address: usize) -> Option<usize> {
        let bytes = read_memory(handle, address, mem::size_of::<usize>())?;
        let mut array = [0_u8; mem::size_of::<usize>()];
        array.copy_from_slice(bytes.get(..mem::size_of::<usize>())?);
        Some(usize::from_ne_bytes(array))
    }

    fn read_u8(handle: HANDLE, address: usize) -> Option<u8> {
        read_memory(handle, address, 1).and_then(|bytes| bytes.first().copied())
    }

    fn read_track_text(handle: HANDLE, address: usize, max_len: usize) -> Option<String> {
        let bytes = read_memory(handle, address, max_len)?;
        super::track_text_from_bytes(&bytes)
    }

    struct ProcessHandle(HANDLE);

    impl ProcessHandle {
        fn open(pid: u32) -> Result<Self, String> {
            let handle =
                unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid) };
            if handle.is_null() {
                Err("could not open rekordbox process for memory reading".to_string())
            } else {
                Ok(Self(handle))
            }
        }
    }

    impl Drop for ProcessHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn find_rekordbox_pid() -> Option<u32> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let _guard = SnapshotHandle(snapshot);
        let mut entry: PROCESSENTRY32W = unsafe { mem::zeroed() };
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;
        if unsafe { Process32FirstW(snapshot, &mut entry) } == 0 {
            return None;
        }
        loop {
            let name = wide_to_string(&entry.szExeFile);
            if name.eq_ignore_ascii_case("rekordbox.exe") {
                return Some(entry.th32ProcessID);
            }
            if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
                break;
            }
        }
        None
    }

    #[allow(dead_code)]
    pub(crate) fn rekordbox_module_base() -> Option<usize> {
        let pid = find_rekordbox_pid()?;
        rekordbox_module_base_for_pid(pid)
    }

    fn rekordbox_module_base_for_pid(pid: u32) -> Option<usize> {
        let snapshot =
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid) };
        if snapshot.is_null() || snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let _guard = SnapshotHandle(snapshot);
        let mut entry: MODULEENTRY32W = unsafe { mem::zeroed() };
        entry.dwSize = mem::size_of::<MODULEENTRY32W>() as u32;
        if unsafe { Module32FirstW(snapshot, &mut entry) } == 0 {
            return None;
        }
        Some(entry.modBaseAddr as usize)
    }

    struct SnapshotHandle(HANDLE);

    impl Drop for SnapshotHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn readable_regions(handle: HANDLE) -> Vec<(usize, usize)> {
        let mut regions = Vec::new();
        let mut address = 0x10000usize;
        let mut info: MEMORY_BASIC_INFORMATION = unsafe { mem::zeroed() };
        while unsafe {
            VirtualQueryEx(
                handle,
                address as _,
                &mut info,
                mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        } != 0
        {
            let base = info.BaseAddress as usize;
            let size = info.RegionSize;
            if size == 0 {
                break;
            }
            if info.State == MEM_COMMIT && is_readable(info.Protect) {
                regions.push((base, size));
            }
            address = base.saturating_add(size);
            if address <= base {
                break;
            }
        }
        regions
    }

    fn is_readable(protect: u32) -> bool {
        protect & PAGE_GUARD == 0
            && protect & PAGE_NOACCESS == 0
            && matches!(
                protect & 0xff,
                PAGE_READONLY
                    | PAGE_READWRITE
                    | PAGE_WRITECOPY
                    | PAGE_EXECUTE_READ
                    | PAGE_EXECUTE_READWRITE
                    | PAGE_EXECUTE_WRITECOPY
            )
    }

    fn read_memory(handle: HANDLE, base: usize, size: usize) -> Option<Vec<u8>> {
        let mut out = vec![0_u8; size];
        let mut read = 0;
        let ok =
            unsafe { ReadProcessMemory(handle, base as _, out.as_mut_ptr() as _, size, &mut read) };
        if ok == 0 || read == 0 {
            None
        } else {
            out.truncate(read);
            Some(out)
        }
    }

    fn extract_ascii_paths(bytes: &[u8]) -> Vec<String> {
        let lower = bytes
            .iter()
            .map(|byte| byte.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let mut paths = Vec::new();
        for ext in AUDIO_EXTENSIONS {
            let needle = ext.as_bytes();
            let mut start = 0;
            while let Some(pos) = find_bytes(&lower[start..], needle).map(|pos| start + pos) {
                if let Some(path) = ascii_path_around(bytes, pos + needle.len()) {
                    paths.push(path);
                }
                start = pos + needle.len();
            }
        }
        paths
    }

    fn ascii_path_around(bytes: &[u8], extension_end: usize) -> Option<String> {
        let mut start = extension_end;
        while start > 0 && is_path_byte(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = extension_end;
        while end < bytes.len() && is_path_byte(bytes[end]) {
            end += 1;
        }
        let value = String::from_utf8_lossy(&bytes[start..end]).to_string();
        plausible_audio_path(value)
    }

    fn extract_utf16_paths(bytes: &[u8]) -> Vec<String> {
        let mut units = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks_exact(2) {
            units.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        let text = String::from_utf16_lossy(&units);
        text.split('\0')
            .filter_map(|part| plausible_audio_path(part.to_string()))
            .collect()
    }

    fn plausible_audio_path(mut value: String) -> Option<String> {
        value = value
            .trim_matches(|ch: char| ch.is_whitespace() || ch == '"' || ch == '\'')
            .to_string();
        let lower = value.to_ascii_lowercase();
        if value.len() < 8
            || !AUDIO_EXTENSIONS.iter().any(|ext| lower.contains(ext))
            || !(value.contains(":\\") || value.contains(":/") || value.contains('/'))
        {
            return None;
        }
        Some(value)
    }

    fn is_path_byte(byte: u8) -> bool {
        matches!(byte, b' '..=b'~') && !matches!(byte, b'"' | b'\'' | b'<' | b'>' | b'|' | b'*')
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn wide_to_string(wide: &[u16]) -> String {
        let end = wide.iter().position(|ch| *ch == 0).unwrap_or(wide.len());
        OsString::from_wide(&wide[..end])
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::{collections::HashMap, mem, process::Command};

    const AUDIO_EXTENSIONS: &[&str] = &[
        ".mp3", ".wav", ".flac", ".aiff", ".aif", ".m4a", ".aac", ".ogg",
    ];
    const RKBX_LINK_OFFSETS: &str = include_str!("rkbx_link_offsets_macos.txt");

    type MachPort = u32;
    type KernReturn = i32;
    type MachVmAddress = u64;
    type MachVmSize = u64;
    const TASK_DYLD_INFO: u32 = 17;
    const TASK_DYLD_INFO_COUNT: u32 =
        (mem::size_of::<TaskDyldInfo>() / mem::size_of::<u32>()) as u32;
    const VM_REGION_BASIC_INFO_64: i32 = 9;
    const VM_REGION_BASIC_INFO_COUNT_64: u32 = 9;
    const VM_PROT_READ: i32 = 1;

    #[repr(C)]
    struct TaskDyldInfo {
        all_image_info_addr: u64,
        all_image_info_size: u64,
        all_image_info_format: i32,
    }

    extern "C" {
        fn mach_task_self() -> MachPort;
        fn task_for_pid(target_tport: MachPort, pid: i32, task: *mut MachPort) -> KernReturn;
        fn task_info(
            target_task: MachPort,
            flavor: u32,
            task_info_out: *mut TaskDyldInfo,
            task_info_count: *mut u32,
        ) -> KernReturn;
        fn mach_vm_read_overwrite(
            target_task: MachPort,
            address: MachVmAddress,
            size: MachVmSize,
            data: MachVmAddress,
            out_size: *mut MachVmSize,
        ) -> KernReturn;
        fn mach_vm_region(
            target_task: MachPort,
            address: *mut MachVmAddress,
            size: *mut MachVmSize,
            flavor: i32,
            info: *mut i32,
            info_count: *mut u32,
            object_name: *mut MachPort,
        ) -> KernReturn;
    }

    pub(crate) fn current_track_path() -> Result<String, String> {
        let pid = rekordbox_pid().ok_or_else(|| "rekordbox process was not found".to_string())?;
        let output = Command::new("lsof")
            .args(["-Fn", "-p", &pid])
            .output()
            .map_err(|err| format!("could not run lsof: {err}"))?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut scores: HashMap<String, usize> = HashMap::new();
        for (index, line) in text.lines().enumerate() {
            let Some(path) = line.strip_prefix('n') else {
                continue;
            };
            if let Some(path) = plausible_audio_path(path.to_string()) {
                *scores.entry(path).or_default() += index + 1;
            }
        }

        scores
            .into_iter()
            .max_by_key(|(path, score)| (*score, path.len()))
            .map(|(path, _)| path)
            .ok_or_else(|| "no audio file found in rekordbox open files".to_string())
    }

    pub(crate) fn current_track() -> Result<super::LiveTrack, String> {
        if let Some(track) = current_track_from_offsets() {
            return Ok(track);
        }
        super::debug_memory_status(
            "macos",
            "offset metadata read failed; falling back to lsof audio path scan",
        );
        current_track_path().map(|file_path| super::LiveTrack {
            file_path,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            bpm: None,
            position_seconds: None,
            duration_seconds: None,
            source: "Rekordbox".to_string(),
            master_verified: false,
        })
    }

    pub(crate) fn current_album_art_data_url() -> Option<String> {
        let pid = rekordbox_pid()?.parse::<i32>().ok()?;
        let task = task_for_rekordbox(pid)?;
        super::best_album_art_from_regions("macos", readable_regions(task), |base, size| {
            read_memory(task, base, size)
        })
    }

    #[derive(Clone)]
    struct Pointer {
        offsets: Vec<usize>,
        final_offset: usize,
    }

    struct OffsetSet {
        version: String,
        masterdeck_index: Pointer,
        current_bpm: Vec<Pointer>,
        sample_position: Vec<Pointer>,
        track_info: Vec<Pointer>,
    }

    fn current_track_from_offsets() -> Option<super::LiveTrack> {
        let Some(pid_text) = rekordbox_pid() else {
            super::debug_memory_status("macos", "rekordbox pid not found");
            return None;
        };
        let Some(pid) = pid_text.parse::<i32>().ok() else {
            super::debug_memory_status("macos", format!("invalid rekordbox pid: {pid_text}"));
            return None;
        };
        let Some(task) = task_for_rekordbox(pid) else {
            super::debug_memory_status(
                "macos",
                format!("task_for_pid failed pid={pid}; memory permission is required"),
            );
            return None;
        };
        let Some(base) = discover_base_address(task) else {
            super::debug_memory_status("macos", "base address discovery failed");
            return None;
        };
        let offset_sets = parse_offsets(RKBX_LINK_OFFSETS);
        super::debug_memory_status(
            "macos",
            format!(
                "pid={pid} base=0x{base:X} offset_sets={}",
                offset_sets.len()
            ),
        );
        for (set_index, offsets) in offset_sets.iter().enumerate() {
            let Some(master_addr) = resolve_pointer(task, base, &offsets.masterdeck_index) else {
                super::debug_memory_status(
                    "macos",
                    format!(
                        "set={} version={} master deck pointer failed",
                        set_index + 1,
                        offsets.version
                    ),
                );
                continue;
            };
            let Some(deck) = read_u8(task, master_addr).map(usize::from) else {
                super::debug_memory_status(
                    "macos",
                    format!(
                        "set={} version={} master deck read failed addr=0x{master_addr:X}",
                        set_index + 1,
                        offsets.version
                    ),
                );
                continue;
            };
            let Some(pointer) = offsets.track_info.get(deck) else {
                super::debug_memory_status(
                    "macos",
                    format!(
                        "set={} version={} deck={} has no track pointer",
                        set_index + 1,
                        offsets.version,
                        deck + 1
                    ),
                );
                continue;
            };
            let Some(track_addr) = resolve_pointer(task, base, pointer) else {
                super::debug_memory_status(
                    "macos",
                    format!(
                        "set={} version={} deck={} track pointer failed",
                        set_index + 1,
                        offsets.version,
                        deck + 1
                    ),
                );
                continue;
            };
            let Some(text) = read_track_text(task, track_addr, super::TRACK_INFO_BYTES) else {
                super::debug_memory_status(
                    "macos",
                    format!(
                        "set={} version={} deck={} text decode failed addr=0x{track_addr:X}",
                        set_index + 1,
                        offsets.version,
                        deck + 1
                    ),
                );
                continue;
            };
            super::debug_memory_text(
                "macos",
                format!("set={} version={}", set_index + 1, offsets.version),
                deck,
                track_addr,
                &text,
            );
            let Some(track) = super::parse_track_info(&text) else {
                super::debug_memory_status(
                    "macos",
                    format!(
                        "set={} version={} deck={} decoded text has no title label",
                        set_index + 1,
                        offsets.version,
                        deck + 1
                    ),
                );
                continue;
            };
            if !track.title.is_empty() {
                return Some(super::LiveTrack {
                    bpm: offsets
                        .current_bpm
                        .get(deck)
                        .and_then(|pointer| read_pointer_value::<4>(task, base, pointer))
                        .map(f32::from_ne_bytes)
                        .filter(|value| value.is_finite() && *value > 0.0),
                    // ponytail: known Rekordbox offsets expose sample position; add per-track sample rate if needed.
                    position_seconds: offsets
                        .sample_position
                        .get(deck)
                        .and_then(|pointer| read_pointer_value::<8>(task, base, pointer))
                        .map(i64::from_ne_bytes)
                        .filter(|value| *value > 0)
                        .map(|value| value as f64 / 44_100.0),
                    source: "Rekordbox".to_string(),
                    master_verified: true,
                    ..track
                });
            }
        }
        None
    }

    fn task_for_rekordbox(pid: i32) -> Option<MachPort> {
        let mut task = 0;
        let result = unsafe { task_for_pid(mach_task_self(), pid, &mut task) };
        (result == 0 && task != 0).then_some(task)
    }

    fn discover_base_address(task: MachPort) -> Option<usize> {
        let mut dyld_info: TaskDyldInfo = unsafe { mem::zeroed() };
        let mut count = TASK_DYLD_INFO_COUNT;
        if unsafe { task_info(task, TASK_DYLD_INFO, &mut dyld_info, &mut count) } != 0 {
            return None;
        }

        let header = read_memory(task, dyld_info.all_image_info_addr as usize, 16)?;
        let image_count = u32::from_ne_bytes(header.get(4..8)?.try_into().ok()?) as usize;
        let info_array_ptr = u64::from_ne_bytes(header.get(8..16)?.try_into().ok()?);
        let entries = read_memory(task, info_array_ptr as usize, image_count.min(512) * 24)?;
        let mut fallback = None;
        for entry in entries.chunks_exact(24) {
            let load_addr = u64::from_ne_bytes(entry.get(..8)?.try_into().ok()?) as usize;
            let path_addr = u64::from_ne_bytes(entry.get(8..16)?.try_into().ok()?) as usize;
            let path = read_c_string(task, path_addr, 1024).unwrap_or_default();
            if fallback.is_none() {
                fallback = Some(load_addr);
            }
            if path.ends_with("/rekordbox.app/Contents/MacOS/rekordbox") {
                super::debug_memory_status(
                    "macos",
                    format!("rekordbox image base=0x{load_addr:X} path={path}"),
                );
                return Some(load_addr);
            }
        }
        fallback
    }

    fn parse_offsets(input: &str) -> Vec<OffsetSet> {
        let lines = input
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect::<Vec<_>>();
        if lines.len() < 6 {
            return Vec::new();
        }

        let mut sets = Vec::new();
        let mut index = 0;
        while index + 1 < lines.len() {
            let version = lines[index].to_string();
            let Some(masterdeck_index) = parse_pointer(lines[index + 1]) else {
                break;
            };
            index += 2;

            let mut current_bpm = Vec::new();
            let mut sample_position = Vec::new();
            let mut track_info = Vec::new();
            while index + 3 < lines.len() && parse_pointer(lines[index]).is_some() {
                if let Some(pointer) = parse_pointer(lines[index]) {
                    current_bpm.push(pointer);
                }
                if let Some(pointer) = parse_pointer(lines[index + 1]) {
                    sample_position.push(pointer);
                }
                if let Some(pointer) = parse_pointer(lines[index + 2]) {
                    track_info.push(pointer);
                }
                index += 4;
            }

            if !track_info.is_empty() {
                sets.push(OffsetSet {
                    version,
                    masterdeck_index,
                    current_bpm,
                    sample_position,
                    track_info,
                });
            }
        }
        sets
    }

    fn parse_pointer(line: &str) -> Option<Pointer> {
        let values = line
            .split_whitespace()
            .map(|part| usize::from_str_radix(part, 16).ok())
            .collect::<Option<Vec<_>>>()?;
        let (final_offset, offsets) = values.split_last()?;
        Some(Pointer {
            offsets: offsets.to_vec(),
            final_offset: *final_offset,
        })
    }

    fn resolve_pointer(task: MachPort, base: usize, pointer: &Pointer) -> Option<usize> {
        let mut address = base;
        for offset in &pointer.offsets {
            address = read_usize(task, address.checked_add(*offset)?)?;
            if address < 0x10000 {
                return None;
            }
        }
        address.checked_add(pointer.final_offset)
    }

    fn read_pointer_value<const N: usize>(
        task: MachPort,
        base: usize,
        pointer: &Pointer,
    ) -> Option<[u8; N]> {
        let address = resolve_pointer(task, base, pointer)?;
        let bytes = read_memory(task, address, N)?;
        bytes.get(..N)?.try_into().ok()
    }

    fn read_usize(task: MachPort, address: usize) -> Option<usize> {
        let bytes = read_memory(task, address, mem::size_of::<usize>())?;
        let mut array = [0_u8; mem::size_of::<usize>()];
        array.copy_from_slice(bytes.get(..mem::size_of::<usize>())?);
        Some(usize::from_ne_bytes(array))
    }

    fn read_u8(task: MachPort, address: usize) -> Option<u8> {
        read_memory(task, address, 1).and_then(|bytes| bytes.first().copied())
    }

    fn read_track_text(task: MachPort, address: usize, max_len: usize) -> Option<String> {
        let bytes = read_memory(task, address, max_len)?;
        super::track_text_from_bytes(&bytes)
    }

    fn read_memory(task: MachPort, address: usize, size: usize) -> Option<Vec<u8>> {
        let mut out = vec![0_u8; size];
        let mut read_size = size as MachVmSize;
        let result = unsafe {
            mach_vm_read_overwrite(
                task,
                address as MachVmAddress,
                size as MachVmSize,
                out.as_mut_ptr() as MachVmAddress,
                &mut read_size,
            )
        };
        if result != 0 || read_size == 0 {
            return None;
        }
        out.truncate(read_size as usize);
        Some(out)
    }

    fn read_c_string(task: MachPort, address: usize, max_len: usize) -> Option<String> {
        let bytes = read_memory(task, address, max_len)?;
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        String::from_utf8(bytes[..end].to_vec()).ok()
    }

    fn readable_regions(task: MachPort) -> Vec<(usize, usize)> {
        let mut regions = Vec::new();
        let mut address = 1 as MachVmAddress;
        loop {
            let mut size = 0 as MachVmSize;
            let mut info = [0_i32; 16];
            let mut count = VM_REGION_BASIC_INFO_COUNT_64;
            let mut object_name = 0;
            let result = unsafe {
                mach_vm_region(
                    task,
                    &mut address,
                    &mut size,
                    VM_REGION_BASIC_INFO_64,
                    info.as_mut_ptr(),
                    &mut count,
                    &mut object_name,
                )
            };
            if result != 0 {
                break;
            }
            if size > 0 && info[0] & VM_PROT_READ != 0 {
                regions.push((address as usize, size as usize));
            }
            let next = address.saturating_add(size);
            if next <= address {
                break;
            }
            address = next;
        }
        regions
    }

    fn rekordbox_pid() -> Option<String> {
        let output = Command::new("pgrep")
            .args(["-x", "rekordbox"])
            .output()
            .ok()?;
        let pid = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::trim)
            .filter(|pid| !pid.is_empty())
            .map(str::to_string);
        if pid.is_some() {
            return pid;
        }

        let output = Command::new("pgrep")
            .args(["-if", "rekordbox"])
            .output()
            .ok()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(str::trim)
            .filter(|pid| !pid.is_empty())
            .map(str::to_string)
    }

    fn plausible_audio_path(value: String) -> Option<String> {
        let value = value.trim().to_string();
        let lower = value.to_ascii_lowercase();
        if value.len() < 8
            || !AUDIO_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
            || !value.contains('/')
        {
            return None;
        }
        Some(value)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    pub(crate) fn current_track_path() -> Result<String, String> {
        Err("rekordbox live path detection is only available on Windows and macOS".to_string())
    }

    pub(crate) fn current_track() -> Result<super::LiveTrack, String> {
        Err("rekordbox live track detection is only available on Windows and macOS".to_string())
    }

    pub(crate) fn current_album_art_data_url() -> Option<String> {
        None
    }
}

pub(crate) use platform::{current_album_art_data_url, current_track};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_japanese_track_labels() {
        let track = parse_track_info(
            "トラックタイトル: BPM (feat. KREVA)\nアーティスト: Kvi Baba, KREVA\nアルバム: BPM (feat. KREVA)",
        )
        .expect("track");

        assert_eq!(track.title, "BPM (feat. KREVA)");
        assert_eq!(track.artist, "Kvi Baba, KREVA");
        assert_eq!(track.album, "BPM (feat. KREVA)");
    }

    #[test]
    fn decodes_utf16le_track_text() {
        let text = "トラックタイトル: BPM (feat. KREVA)\nアーティスト: Kvi Baba, KREVA";
        let mut bytes = text
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        bytes.extend_from_slice(&[0, 0]);

        assert_eq!(track_text_from_bytes(&bytes).as_deref(), Some(text));
    }

    #[test]
    fn sanitizes_debug_memory_text() {
        let text = format!("a\0b\n{}", "x".repeat(1200));
        let sanitized = sanitize_debug_text(&text);

        assert!(sanitized.starts_with("a\\0b | "));
        assert!(sanitized.ends_with("..."));
        assert!(sanitized.chars().count() <= 1003);
    }

    #[test]
    fn extracts_png_album_art_from_memory() {
        let mut bytes = b"before".to_vec();
        bytes.extend_from_slice(PNG_SIGNATURE);
        bytes.extend_from_slice(&16_u32.to_be_bytes());
        bytes.extend_from_slice(b"tEXt");
        bytes.extend_from_slice(&[1; 16]);
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(b"IEND");
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        bytes.extend_from_slice(b"after");

        let (_, url) = album_art_data_url_from_bytes(&bytes).expect("png");
        assert!(url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn extracts_jpeg_album_art_from_memory() {
        let mut bytes = b"before".to_vec();
        bytes.extend_from_slice(b"\xff\xd8");
        bytes.extend_from_slice(&[1; 64]);
        bytes.extend_from_slice(b"\xff\xd9");
        bytes.extend_from_slice(b"after");

        let (_, url) = album_art_data_url_from_bytes(&bytes).expect("jpeg");
        assert!(url.starts_with("data:image/jpeg;base64,"));
    }
}
