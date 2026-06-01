import { create } from 'zustand';
import type { AppStatusDto, CustomerDto, ProductDto } from '../shared/types';

type AppStore = {
  status?: AppStatusDto;
  products: ProductDto[];
  customers: CustomerDto[];
  lastCategory?: string;
  productFilter?: { onlyLowStock?: boolean };
  setStatus: (status: AppStatusDto) => void;
  setProducts: (products: ProductDto[]) => void;
  setCustomers: (customers: CustomerDto[]) => void;
  setLastCategory: (category: string) => void;
  setProductFilter: (filter: { onlyLowStock?: boolean }) => void;
};

export const useAppStore = create<AppStore>((set) => ({
  products: [],
  customers: [],
  setStatus: (status) => set({ status }),
  setProducts: (products) => set({ products }),
  setCustomers: (customers) => set({ customers }),
  setLastCategory: (lastCategory) => set({ lastCategory }),
  setProductFilter: (productFilter) => set({ productFilter })
}));
