/**
 * The one place the compile server's port is decided.
 *
 * Three processes have to agree on it: the server that listens, the Vite proxy that forwards `/api`
 * to it, and `dev:all` which starts both. Reading `PORT` here moves all three together, where
 * reading it in only one of them would leave the proxy pointed at a port nothing is serving.
 */
export const COMPILE_PORT = Number(process.env.PORT ?? 5174);
