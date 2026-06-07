import { Tag } from 'antd';

export function statusTag(status: string) {
  if (status === 'success' || status === 'normal') {
    return <Tag color="green">{status}</Tag>;
  }
  if (status === 'warning') {
    return <Tag color="orange">{status}</Tag>;
  }
  if (status === 'voided' || status === 'failed' || status === 'error') {
    return <Tag color="red">{status}</Tag>;
  }
  return <Tag>{status}</Tag>;
}
