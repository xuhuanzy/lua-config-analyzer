---@alias array<T> T[]

---@alias list<T> T[]

---@alias set<T> table<T, any>

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

---@class ConfigTable

---@attribute v.index(id: string)

-- 标记枚举为位域, 位域的值为2的幂次.
---@attribute flags()

-- 检查某字段是否为某配置表的合法 key.
--
-- ### 参数:
--
-- - `tableName`: 配置表名称
--
---@attribute v.ref(tableName: string)

-- 检查`list<bean>`与`array<bean>`内指定字段的值是否唯一.
--
-- ### 参数:
--
-- - `id`: 指定字段名称
---@attribute v.index(id: string)


-- 定义配置表的索引字段列表, 可以有多个索引字段.
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
