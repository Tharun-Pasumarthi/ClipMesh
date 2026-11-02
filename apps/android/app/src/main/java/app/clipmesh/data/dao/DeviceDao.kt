package app.clipmesh.data.dao

import androidx.room.*
import app.clipmesh.data.model.DeviceEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface DeviceDao {
    @Query("SELECT * FROM devices WHERE isRevoked = 0 ORDER BY lastSeenMs DESC")
    fun getActiveDevices(): Flow<List<DeviceEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsertDevice(device: DeviceEntity)

    @Query("UPDATE devices SET isRevoked = 1 WHERE id = :deviceId")
    suspend fun revokeDevice(deviceId: String)

    @Query("DELETE FROM devices WHERE id = :deviceId")
    suspend fun deleteDevice(deviceId: String)
}

