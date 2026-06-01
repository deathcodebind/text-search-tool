import Login from "./routes/Login.svelte";
import Pull from "./routes/Pull.svelte";
import Detail from "./routes/Detail.svelte";

export const routes = {
  "/": Login,
  "/login": Login,
  "/pull": Pull,
  "/detail": Detail,
  "/detail/:sourceId": Detail,
  "*": Login,
};
