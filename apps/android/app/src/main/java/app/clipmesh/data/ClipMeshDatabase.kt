package app.clipmesh.data

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import app.clipmesh.data.dao.ClipDao
import app.clipmesh.data.dao.DeviceDao
import app.clipmesh.data.model.ClipEntity
import app.clipmesh.data.model.DeviceEntity

@Database(entities = [ClipEntity::class, DeviceEntity::class], version = 1, exportSchema = false)
abstract class ClipMeshDatabase : RoomDatabase() {
    abstract fun clipDao(): ClipDao
    abstract fun deviceDao(): DeviceDao

    companion object {
        @Volatile
        private var INSTANCE: ClipMeshDatabase? = null

        fun getDatabase(context: Context): ClipMeshDatabase {
            return INSTANCE ?: synchronized(this) {
                val instance = Room.databaseBuilder(
                    context.applicationContext,
                    ClipMeshDatabase::class.java,
                    "clipmesh_local.db"
                ).build()
                INSTANCE = instance
                instance
            }
        }
    }
}
