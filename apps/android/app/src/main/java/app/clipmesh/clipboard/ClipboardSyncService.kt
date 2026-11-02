package app.clipmesh.clipboard

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat

/**
 * Foreground Service for Clipboard Synchronization on Android.
 *
 * NOTE ON ANDROID 10+ BACKGROUND EXECUTION:
 * Android 10 (API level 29) prohibits apps in the background from accessing the clipboard
 * unless they are the default input method editor (IME) or currently in the foreground.
 *
 * ClipMesh respects user privacy and OS platform security constraints:
 * - Sync is conducted while this Foreground Service has an active notification.
 * - Quick Settings Tile and App Shortcuts allow explicit, user-initiated clipboard pulls.
 */
class ClipboardSyncService : Service() {

    private lateinit var clipboardManager: ClipboardManager
    private val CHANNEL_ID = "clipmesh_sync_channel"
    private val NOTIFICATION_ID = 4041

    private val clipChangedListener = ClipboardManager.OnPrimaryClipChangedListener {
        onClipboardChanged()
    }

    override fun onCreate() {
        super.onCreate()
        clipboardManager = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        clipboardManager.addPrimaryClipChangedListener(clipChangedListener)
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, buildForegroundNotification())
    }

    override fun onDestroy() {
        clipboardManager.removePrimaryClipChangedListener(clipChangedListener)
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun onClipboardChanged() {
        val clip: ClipData? = clipboardManager.primaryClip
        if (clip != null && clip.itemCount > 0) {
            val item = clip.getItemAt(0)
            val text = item.text?.toString()
            if (!text.isNullOrBlank()) {
                // Pass text to Rust Core via UniFFI bridge for encryption and relay delivery
                // NativeCore.publishClipboard(text)
            }
        }
    }

    private fun buildForegroundNotification(): Notification {
        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("ClipMesh Sync Active")
            .setContentText("Listening for clipboard updates securely.")
            .setSmallIcon(android.R.drawable.ic_menu_agenda)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "ClipMesh Background Sync",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Keeps device connected to the private ClipMesh relay."
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }
}

