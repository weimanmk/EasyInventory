import { SearchOutlined, ShoppingCartOutlined } from '@ant-design/icons';
import { App, Button, Input, InputNumber, List, Modal, Space, Tag, Typography } from 'antd';
import { useMemo, useState } from 'react';
import { api } from '../api/inventory';
import { money, qty, uniqueValues } from '../shared/format';
import type { CustomerDto, OrderLine, ProductDto, QuotePreviewDto } from '../shared/types';
import { useAppStore } from '../store/appStore';

type Props = {
  open: boolean;
  customer?: CustomerDto;
  onClose: () => void;
  onAdd: (line: OrderLine) => void;
};

export default function ProductPickerModal({ open, customer, onClose, onAdd }: Props) {
  const { message } = App.useApp();
  const { products, lastCategory, setLastCategory } = useAppStore();
  const categories = useMemo(() => ['全部', ...uniqueValues(products, (item) => item.category)], [products]);
  const [category, setCategory] = useState(lastCategory ?? '全部');
  const [keyword, setKeyword] = useState('');
  const [scanCode, setScanCode] = useState('');
  const [selected, setSelected] = useState<ProductDto>();
  const [quantity, setQuantity] = useState(1);
  const [unitPrice, setUnitPrice] = useState<number | null>(null);
  const [remark, setRemark] = useState('');
  const [preview, setPreview] = useState<QuotePreviewDto>();

  const filtered = useMemo(() => {
    return products.filter((item) => {
      const matchCategory = category === '全部' || item.category === category;
      const matchKeyword =
        !keyword ||
        item.name.includes(keyword) ||
        item.barcode?.includes(keyword) ||
        item.category.includes(keyword);
      return matchCategory && matchKeyword;
    });
  }, [category, keyword, products]);

  async function choose(product: ProductDto) {
    setSelected(product);
    const price = unitPrice ?? product.defaultPrice ?? 0;
    setUnitPrice(price);
    await refreshPreview(product, quantity, price);
  }

  async function refreshPreview(product = selected, nextQuantity = quantity, nextPrice = unitPrice ?? undefined) {
    if (!product || !customer) {
      setPreview(undefined);
      return;
    }
    try {
      const data = await api.previewQuote({
        customerId: customer.id,
        productId: product.id,
        quantity: nextQuantity,
        manualPrice: nextPrice,
        orderDate: new Date().toISOString().slice(0, 10)
      });
      setPreview(data);
      if (unitPrice === null) {
        setUnitPrice(data.unitPrice);
      }
    } catch (error) {
      message.warning(error instanceof Error ? error.message : '报价预览失败');
    }
  }

  async function addSelected() {
    if (!selected) {
      message.warning('请选择商品');
      return;
    }
    const price = unitPrice ?? selected.defaultPrice ?? preview?.unitPrice ?? 0;
    const amount = Number((quantity * price).toFixed(2));
    onAdd({
      key: `${selected.id}-${Date.now()}`,
      productId: selected.id,
      productName: selected.name,
      category: selected.category,
      barcode: selected.barcode,
      currentStock: selected.currentStock,
      quantity,
      unitPrice: price,
      amount,
      ruleMessage: preview?.message,
      remark,
      preview
    });
    setQuantity(1);
    setRemark('');
    message.success('已加入出库单');
  }

  async function scanAndAdd() {
    const barcode = scanCode.trim();
    if (!barcode) {
      return;
    }
    try {
      const product = await api.findProductByBarcode(barcode);
      if (!product) {
        message.warning(`未找到条码：${barcode}`);
        return;
      }
      setSelected(product);
      const data = customer
        ? await api.previewQuote({
          customerId: customer.id,
          productId: product.id,
          quantity,
          orderDate: new Date().toISOString().slice(0, 10)
        })
        : undefined;
      const price = data?.unitPrice ?? product.defaultPrice ?? 0;
      setUnitPrice(price);
      setPreview(data);
      onAdd({
        key: `${product.id}-${Date.now()}`,
        productId: product.id,
        productName: product.name,
        category: product.category,
        barcode: product.barcode,
        currentStock: product.currentStock,
        quantity,
        unitPrice: price,
        amount: Number((quantity * price).toFixed(2)),
        ruleMessage: data?.message,
        remark,
        preview: data
      });
      setScanCode('');
      setRemark('');
      message.success(`扫码加入：${product.name}`);
    } catch (error) {
      message.warning(error instanceof Error ? error.message : '扫码加入失败');
    }
  }

  return (
    <Modal
      title="选择商品"
      open={open}
      onCancel={onClose}
      footer={null}
      width={1120}
      styles={{ body: { height: 'min(68vh, 620px)', padding: 0, overflow: 'hidden' } }}
      destroyOnClose={false}
    >
      <div className="product-picker">
        <div className="product-picker-main">
          <div className="panel product-picker-categories">
            <Typography.Text type="secondary">商品类别</Typography.Text>
            <List
              size="small"
              dataSource={categories}
              renderItem={(item) => (
                <List.Item
                  style={{ cursor: 'pointer', fontWeight: item === category ? 700 : 400 }}
                  onClick={() => {
                    setCategory(item);
                    setLastCategory(item);
                  }}
                >
                  {item}
                </List.Item>
              )}
            />
          </div>
          <div className="panel product-picker-products">
            <Space className="product-picker-search">
              <Input
                autoFocus
                placeholder="扫码后回车自动加入"
                value={scanCode}
                onChange={(event) => setScanCode(event.target.value)}
                onPressEnter={() => void scanAndAdd()}
                style={{ width: 220 }}
              />
              <Input
                prefix={<SearchOutlined />}
                allowClear
                placeholder="搜索商品名或条码"
                value={keyword}
                onChange={(event) => setKeyword(event.target.value)}
                style={{ width: 360 }}
              />
              <Tag color={customer ? 'blue' : 'orange'}>{customer ? `当前客户：${customer.name}` : '请先选择客户'}</Tag>
            </Space>
            <div className="ag-theme-quartz product-picker-list">
              <List
                dataSource={filtered}
                grid={{ gutter: 8, column: 3 }}
                renderItem={(item) => (
                <List.Item>
                  <button
                    className={`product-card ${selected?.id === item.id ? 'active' : ''}`}
                    onClick={() => void choose(item)}
                    onDoubleClick={() => void choose(item).then(addSelected)}
                  >
                    <strong>{item.name}</strong>
                    <span>{item.category} · {item.barcode || '无条码'}</span>
                    <span>库存 {qty(item.currentStock)} · 默认价 {money(item.defaultPrice)}</span>
                    {item.currentStock <= item.safetyStock && <Tag color="red">低库存</Tag>}
                  </button>
                </List.Item>
              )}
            />
            </div>
          </div>
        </div>
        <div className="panel product-picker-actions">
          <Typography.Text strong className="product-picker-selected">{selected?.name ?? '未选择商品'}</Typography.Text>
          <InputNumber
            min={0.01}
            value={quantity}
            addonBefore="数量"
            className="product-picker-number"
            onChange={(value) => {
              const next = Number(value ?? 1);
              setQuantity(next);
              void refreshPreview(selected, next, unitPrice ?? undefined);
            }}
          />
          <InputNumber
            min={0}
            value={unitPrice}
            addonBefore="价格"
            className="product-picker-number"
            onChange={(value) => {
              const next = Number(value ?? 0);
              setUnitPrice(next);
              void refreshPreview(selected, quantity, next);
            }}
          />
          <Input
            placeholder="备注"
            value={remark}
            onChange={(event) => setRemark(event.target.value)}
            className="product-picker-remark"
            onPressEnter={() => void addSelected()}
          />
          <Tag color="green" className="product-picker-total">金额 {money((unitPrice ?? 0) * quantity)}</Tag>
          {preview?.message && <Tag color="cyan" className="product-picker-message">{preview.message}</Tag>}
          <Button type="primary" icon={<ShoppingCartOutlined />} onClick={() => void addSelected()} className="product-picker-add">
            加入
          </Button>
        </div>
      </div>
    </Modal>
  );
}
