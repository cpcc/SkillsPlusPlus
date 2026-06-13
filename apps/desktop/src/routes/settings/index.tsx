import { useAppInfo } from "../../hooks/use-app-info";

export default function SettingsPage() {
  const { data, isLoading, error } = useAppInfo();

  return (
    <div>
      <h2 className="text-xl font-semibold text-gray-900">设置</h2>
      <p className="mt-2 text-gray-500">来源站配置、缓存管理与日志。</p>

      <div className="mt-6 rounded-lg border border-gray-200 bg-white p-4">
        <h3 className="text-sm font-medium text-gray-700">应用信息</h3>
        {isLoading && <p className="mt-2 text-sm text-gray-400">加载中...</p>}
        {error && (
          <p className="mt-2 text-sm text-red-500">
            加载失败：{String(error)}
          </p>
        )}
        {data && (
          <dl className="mt-2 space-y-1 text-sm text-gray-600">
            <div className="flex gap-2">
              <dt className="font-medium">版本：</dt>
              <dd>{data.version}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="font-medium">平台：</dt>
              <dd>{data.platform}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="font-medium">数据库：</dt>
              <dd className="truncate font-mono text-xs">{data.dbPath}</dd>
            </div>
          </dl>
        )}
      </div>
    </div>
  );
}
