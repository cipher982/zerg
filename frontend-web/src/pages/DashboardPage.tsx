import { useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchAgents } from "../services/api";

export default function DashboardPage() {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["agents"],
    queryFn: fetchAgents
  });

  useEffect(() => {
    const ws = new WebSocket(`${window.location.origin.replace("http", "ws")}/api/ws`);
    ws.onmessage = () => {
      // TODO: hydrate from typed events.
      refetch();
    };
    return () => ws.close();
  }, [refetch]);

  if (isLoading) {
    return <div>Loading agents…</div>;
  }

  if (error) {
    return <div>Failed to load agents</div>;
  }

  return (
    <div>
      <header>
        <h1>Agent Dashboard (React Prototype)</h1>
      </header>
      <section>
        <ul>
          {data?.map((agent) => (
            <li key={agent.id}>
              <strong>{agent.name}</strong> – {agent.status}
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
