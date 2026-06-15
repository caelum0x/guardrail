export function TxHashLink({ hash }: { hash: string }) {
  return (
    <a className="mono link" href={`https://bscscan.com/tx/${hash}`}>
      {hash}
    </a>
  );
}
