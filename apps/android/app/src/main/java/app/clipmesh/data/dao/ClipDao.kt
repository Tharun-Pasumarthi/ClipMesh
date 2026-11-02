package app.clipmesh.data.dao

import androidx.room.*
import app.clipmesh.data.model.ClipEntity
import kotlinx.coroutines.flow.Flow

@Dao
interface ClipDao {
    @Query("SELECT * FROM clips ORDER BY isPinned DESC, timestampMs DESC LIMIT :limit")
    fun getRecentClips(limit: Int = 100): Flow<List<ClipEntity>>

    @Query("SELECT * FROM clips WHERE body LIKE '%' || :query || '%' OR title LIKE '%' || :query || '%' ORDER BY timestampMs DESC")
    fun searchClips(query: String): Flow<List<ClipEntity>>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertClip(clip: ClipEntity): Long

    @Query("UPDATE clips SET isPinned = NOT isPinned WHERE id = :id")
    suspend fun togglePin(id: Long)

    @Query("DELETE FROM clips WHERE id = :id")
    suspend fun deleteClip(id: Long)

    @Query("DELETE FROM clips WHERE isPinned = 0")
    suspend fun clearUnpinned(): Int

    @Query("DELETE FROM clips WHERE isPinned = 0 AND timestampMs < :cutoffMs")
    suspend fun cleanupRetention(cutoffMs: Long): Int
}
