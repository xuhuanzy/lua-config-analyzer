---@alias array<T> T[]

---@alias list<T> T[]

---@alias set<T> T[]

---@alias map<K,V> table<K,V>

-- 0 - 255
---@alias byte integer

-- -32768 - 32767
---@alias short integer

---@alias long integer

---@alias float number

---@alias double number

-- 类型为`long`, 值为自 UTC 1970-01-01 00:00:00 以来的秒数
---@alias datetime number

---@class Bean
---@field __tag__ string? # 可以有0到多个tag, 用于标识记录, 每个tag之间使用`,`分隔.

---@class ConfigTable

-- 标记枚举为位域, 位域的值为2的幂次.
---@attribute flags()

-- 定义配置表的索引(主键)字段列表, 可以有多个索引字段.
--
-- 如果配置表的索引字段列表为空且配置表模式为"map", 则使用值类型的第一个字段作为索引字段.
--
-- ### 参数:
--
-- - `indexs`: 索引字段列表, 可以是字符串或字符串数组.
-- - `mode`: 索引模式, 仅在提供多个索引字段时有效, 可以是"union"(联合)或"solo"(独立), 默认值为"union".
---@attribute t.index(indexs: string|string[], mode?: "union" | "solo")


-- 配置表模式.
--
-- - "map": 普通表, 默认值.
-- - "list": 列表, 允许多主键.
-- - "singleton": 单例.
---@attribute t.mode(mode: "map" | "list" | "singleton")

--#region validator

-- 检查某字段是否为某配置表的合法 key.
--
-- ### 参数:
--
-- - `tableName`: 配置表名称, 必须是`ConfigTable`的子类.
-- - `key`: 指定主键名称. 如果不提供, 则自动使用配置表的主键.
--
---@attribute v.ref(tableName: string, key?: string)

-- 检查`array<Bean>`/`list<Bean>`/`set<Bean>`内指定字段的值是否唯一.
--
-- 被检查的元素类型必须为`Bean`.
--
-- ### 参数:
--
-- - `key`: 指定字段名称
---@attribute v.index(key: string)

--#endregion
