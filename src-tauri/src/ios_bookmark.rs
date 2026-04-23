// iOS security-scoped bookmark persistence.
//
// On iOS, access to folders outside the app sandbox (including iCloud Drive folders
// picked via the Files picker) requires a security-scoped URL. The OS grants access
// for the current session when the user picks a folder, but that access does NOT
// survive app restarts unless we serialise the URL as a bookmark blob and store it.
//
// Flow:
//   1. User picks a sync folder (pick_sync_folder command) → call save_bookmark(path)
//   2. On next launch → call restore_bookmark() to re-acquire access and get the path
//   3. On app exit/background → call stop_accessing() to release the security scope

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{
    NSData, NSDictionary, NSError, NSString, NSURL, NSURLBookmarkCreationOptions,
    NSURLBookmarkResolutionOptions, NSUserDefaults,
};
use std::ptr;
use std::sync::Mutex;

const BOOKMARK_KEY: &str = "sync_folder_bookmark_v1";

// Holds the active security-scoped URL so we can call stopAccessingSecurityScopedResource.
static ACTIVE_URL: Mutex<Option<Retained<NSURL>>> = Mutex::new(None);

/// Serialize the given path as a security-scoped bookmark and store it in NSUserDefaults.
/// Call this immediately after the user picks a sync folder.
pub fn save_bookmark(path: &str) -> Result<(), String> {
    unsafe {
        let url = NSURL::fileURLWithPath(&NSString::from_str(path));
        let mut error: *mut NSError = ptr::null_mut();

        let bookmark_data = url.bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
            NSURLBookmarkCreationOptions::WithSecurityScope,
            None,
            None,
            &mut error,
        );

        if !error.is_null() {
            let desc = (*error).localizedDescription();
            return Err(format!("Failed to create bookmark: {desc}"));
        }

        let Some(data) = bookmark_data else {
            return Err("bookmarkDataWithOptions returned nil".to_string());
        };

        let defaults = NSUserDefaults::standardUserDefaults();
        defaults.setObject_forKey(Some(&data), &NSString::from_str(BOOKMARK_KEY));
        defaults.synchronize();
        Ok(())
    }
}

/// Resolve the stored bookmark and begin accessing the security-scoped resource.
/// Returns the resolved path string, or None if no bookmark is stored or it is stale.
/// Call this at app startup before attempting any file I/O on the sync folder.
pub fn restore_bookmark() -> Option<String> {
    unsafe {
        let defaults = NSUserDefaults::standardUserDefaults();
        let data = defaults.dataForKey(&NSString::from_str(BOOKMARK_KEY))?;

        let mut is_stale: bool = false;
        let mut error: *mut NSError = ptr::null_mut();

        let url = NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
            &data,
            NSURLBookmarkResolutionOptions::WithSecurityScope,
            None,
            &mut is_stale,
            &mut error,
        );

        if !error.is_null() || url.is_none() {
            // Stale or broken bookmark — clear it so we don't keep trying
            defaults.removeObjectForKey(&NSString::from_str(BOOKMARK_KEY));
            return None;
        }

        let url = url?;

        // If the bookmark is stale we should refresh it, but still use it this session
        if is_stale {
            if let Some(path) = url_to_path_string(&url) {
                let _ = save_bookmark(&path);
            }
        }

        url.startAccessingSecurityScopedResource();
        let path = url_to_path_string(&url);

        // Keep the URL alive so we can stop accessing later
        *ACTIVE_URL.lock().unwrap() = Some(url);

        path
    }
}

/// Release the security-scoped resource. Call when the app is going to background or exit.
pub fn stop_accessing() {
    if let Some(url) = ACTIVE_URL.lock().unwrap().take() {
        unsafe {
            url.stopAccessingSecurityScopedResource();
        }
    }
}

fn url_to_path_string(url: &NSURL) -> Option<String> {
    unsafe {
        url.path().map(|s| s.to_string())
    }
}
