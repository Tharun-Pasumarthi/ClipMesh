package app.clipmesh.data.model

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "devices")
data class DeviceEntity(
    @PrimaryKey
    val id: String,
    val name: String,
    val platform: String,
    val fingerprint: String,
    val isRevoked: Boolean,
    val lastSeenMs: Long
)
