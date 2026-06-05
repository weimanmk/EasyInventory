import { create } from 'zustand';
import type {
  AppStatusDto,
  CustomerDto,
  FeatureFlagsDto,
  MerchantProfileDto,
  ProductDto,
  SetupStatusDto,
  TermSettingsDto
} from '../shared/types';

export const defaultTerms: TermSettingsDto = {
  customer: '客户',
  region: '地区',
  product: '商品',
  category: '类别',
  rule: '价格规则',
  credit: '返利额度',
  guestCustomer: '散客'
};

export const defaultFeatures: FeatureFlagsDto = {
  supplierLedger: true,
  customerRules: true,
  monthlyCredit: true,
  receivables: true,
  productRanking: true,
  customerAnalysis: true,
  inventoryControl: true,
  diagnostics: true
};

export const defaultMerchant: MerchantProfileDto = {
  name: '我的商行'
};

type AppStore = {
  status?: AppStatusDto;
  setupStatus?: SetupStatusDto;
  merchant: MerchantProfileDto;
  terms: TermSettingsDto;
  features: FeatureFlagsDto;
  products: ProductDto[];
  customers: CustomerDto[];
  lastCategory?: string;
  productFilter?: { onlyLowStock?: boolean };
  setStatus: (status: AppStatusDto) => void;
  setSetupStatus: (setupStatus: SetupStatusDto) => void;
  setMerchant: (merchant: MerchantProfileDto) => void;
  setTerms: (terms: TermSettingsDto) => void;
  setFeatures: (features: FeatureFlagsDto) => void;
  setProducts: (products: ProductDto[]) => void;
  setCustomers: (customers: CustomerDto[]) => void;
  setLastCategory: (category: string) => void;
  setProductFilter: (filter: { onlyLowStock?: boolean }) => void;
};

export const useAppStore = create<AppStore>((set) => ({
  merchant: defaultMerchant,
  terms: defaultTerms,
  features: defaultFeatures,
  products: [],
  customers: [],
  setStatus: (status) => set({ status }),
  setSetupStatus: (setupStatus) => set({ setupStatus }),
  setMerchant: (merchant) => set({ merchant }),
  setTerms: (terms) => set({ terms }),
  setFeatures: (features) => set({ features }),
  setProducts: (products) => set({ products }),
  setCustomers: (customers) => set({ customers }),
  setLastCategory: (lastCategory) => set({ lastCategory }),
  setProductFilter: (productFilter) => set({ productFilter })
}));
