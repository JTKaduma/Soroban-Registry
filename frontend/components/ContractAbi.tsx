'use client';

import { useState, useEffect } from 'react';

interface AbiInput {
  name: string;
  value: { type: string };
}

interface AbiOutput {
  type: string;
}

interface AbiItem {
  type: string;
  name?: string;
  doc?: string;
  inputs?: AbiInput[];
  outputs?: AbiOutput[];
}

export default function ContractAbi({ contractId }: { contractId: string }) {
  const [specs, setSpecs] = useState<AbiItem[]>([]);

  useEffect(() => {
    fetch(`${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001'}/api/contracts/${contractId}/abi`)
      .then(res => res.ok ? res.json() : [])
      .then(setSpecs)
      .catch(() => setSpecs([]));
  }, [contractId]);

  const functions = specs.filter(s => s.type === 'function');

  if (!functions.length) return <div>No ABI available</div>;

  return (
    <div className="space-y-4">
      <h3 className="text-xl font-bold">Functions</h3>
      {functions.map((fn, i) => (
        <div key={i} className="border rounded p-4">
          <h4 className="font-mono font-bold text-blue-600">{fn.name}</h4>
          {fn.doc && <p className="text-sm text-gray-600 mt-1">{fn.doc}</p>}

          <div className="mt-2">
            <span className="text-sm font-semibold">Parameters:</span>
            {fn.inputs?.length ? (
              <ul className="ml-4 text-sm">
                {fn.inputs.map((inp, j: number) => (
                  <li key={j}><code>{inp.name}: {inp.value.type}</code></li>
                ))}
              </ul>
            ) : <span className="text-sm ml-2">None</span>}
          </div>

          <div className="mt-2">
            <span className="text-sm font-semibold">Returns:</span>
            <span className="text-sm ml-2">
              {fn.outputs?.length ? fn.outputs.map((o) => o.type).join(', ') : 'void'}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}
