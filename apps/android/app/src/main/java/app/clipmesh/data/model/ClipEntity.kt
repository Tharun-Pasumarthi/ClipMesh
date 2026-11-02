package app.clipmesh.data.model

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "clips")
data class ClipEntity(
    @PrimaryKey(autoGenerate = true)
    val id: Long = 0,
    val eventId: String,
    val kind: String,
    val title: String,
    val body: String,
    val isPinned: Boolean,
    val originDevice: String,
    val timestampMs: Long
)

