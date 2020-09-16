// This file is part of Substrate.

// Copyright (C) 2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::pallet::Def;
use proc_macro2::Span;
use syn::spanned::Spanned;

/// * implement the trait `sp_runtime::BuildModuleGenesisStorage` using `__InherentHiddenInstance`
///   when needed
/// * also implement the helper function `build_storage` and `assimilate_storage`.
/// * add #[cfg(features = "std")] to GenesisBuild implementation.
pub fn expand_genesis_build(def: &mut Def) -> proc_macro2::TokenStream {
	let genesis_config = if let Some(genesis_config) = &def.genesis_config {
		genesis_config
	} else {
		return Default::default()
	};

	let scrate = &def.scrate();
	let type_impl_gen = &def.type_impl_generics();
	let type_use_gen = &def.type_use_generics();
	let trait_use_gen = if def.trait_.has_instance {
		quote::quote!(T, I)
	} else {
		let inherent_instance = syn::Ident::new(crate::INHERENT_INSTANCE_NAME, Span::call_site());
		quote::quote!(T, #inherent_instance)
	};
	let gen_cfg_ident = &genesis_config.genesis_config;

	let (
		gen_cfg_impl_gen,
		gen_cfg_use_gen
	) = match (genesis_config.has_trait, genesis_config.has_instance) {
		(false, false) => (quote::quote!(), quote::quote!()),
		(true, false) => (quote::quote!(T: Trait), quote::quote!(T)),
		(true, true) => (quote::quote!(T: Trait<I>, I: Instance), quote::quote!(T, I)),
		(false, true) => unreachable!("Checked by def parser"),
	};

	let (fn_impl_gen, fn_use_gen) = match (genesis_config.has_trait, def.trait_.has_instance) {
		(false, false) => (quote::quote!(T: Trait), quote::quote!(T)),
		(false, true) => (quote::quote!(T: Trait<I>, I: Instance), quote::quote!(T, I)),
		(true, _) => (quote::quote!(), quote::quote!()),
	};

	let genesis_build = def.genesis_build.as_ref().expect("Checked by def parser");
	let genesis_build_item = &mut def.item.content.as_mut()
		.expect("Checked by def parser").1[genesis_build.index];

	let genesis_build_item_impl = if let syn::Item::Impl(impl_) = genesis_build_item {
		impl_
	} else {
		unreachable!("Checked by genesis_build parser")
	};

	genesis_build_item_impl.attrs.push(syn::parse_quote!( #[cfg(feature = "std")] ));
	let where_clause = &genesis_build.where_clause;

	quote::quote_spanned!(genesis_build_item.span() =>
		#[cfg(feature = "std")]
		impl<#type_impl_gen> #scrate::sp_runtime::BuildModuleGenesisStorage<#trait_use_gen>
			for #gen_cfg_ident<#gen_cfg_use_gen> #where_clause
		{
			fn build_module_genesis_storage(
				&self,
				storage: &mut #scrate::sp_runtime::Storage,
			) -> std::result::Result<(), std::string::String> {
				#scrate::BasicExternalities::execute_with_storage(storage, || {
					<Self as #scrate::traits::GenesisBuild<#type_use_gen>>::build(self);
					Ok(())
				})
			}
		}

		#[cfg(feature = "std")]
		impl<#gen_cfg_impl_gen> #gen_cfg_ident<#gen_cfg_use_gen> {
			/// Build the storage using `build` inside default storage.
			pub fn build_storage<#fn_impl_gen>(
				&self
			) -> sp_std::result::Result<sp_runtime::Storage, String> #where_clause
			{
				let mut storage = Default::default();
				self.assimilate_storage::<#fn_use_gen>(&mut storage)?;
				Ok(storage)
			}

			/// Assimilate the storage for this module into pre-existing overlays.
			pub fn assimilate_storage<#fn_impl_gen>(
				&self,
				storage: &mut sp_runtime::Storage,
			) -> sp_std::result::Result<(), String> #where_clause
			{
				#scrate::BasicExternalities::execute_with_storage(storage, || {
					<Self as #scrate::traits::GenesisBuild<#type_use_gen>>::build(self);
					Ok(())
				})
			}
		}
	)
}
