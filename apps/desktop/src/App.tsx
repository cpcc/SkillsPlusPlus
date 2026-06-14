import { AppRoutes } from "./routes/index";
import { useUpdateCheck } from "./hooks/use-update-check";

export default function App() {
  // 在根组件挂一次：触发请求并填充 Query 缓存，
  // 子组件（SideNav、设置页）直接读缓存，不会重复打 API。
  useUpdateCheck();
  return <AppRoutes />;
}
