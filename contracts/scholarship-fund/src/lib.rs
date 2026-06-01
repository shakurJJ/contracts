#![no_std]
use soroban_sdk::{contract,contracterror,contractimpl,contracttype,symbol_short,Address,Env,String};
#[contracterror]
#[derive(Copy,Clone,Debug,Eq,PartialEq)]
#[repr(u32)]
pub enum Error{NotInitialized=1,AlreadyInitialized=2,Unauthorized=3,ZeroAmount=4,InsufficientFunds=5}
#[contracttype]
pub enum DataKey{Admin,PoolBalance,Deposit(Address)}
#[contracttype]
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct FundStats{pub pool_balance:i128}
#[contract]
pub struct ScholarshipFundContract;
#[contractimpl]
impl ScholarshipFundContract{
    pub fn initialize(env:Env,admin:Address)->Result<(),Error>{
        if env.storage().instance().has(&DataKey::Admin){return Err(Error::AlreadyInitialized);}
        env.storage().instance().set(&DataKey::Admin,&admin);
        env.storage().instance().set(&DataKey::PoolBalance,&0i128);
        Ok(())
    }
    pub fn deposit(env:Env,depositor:Address,amount:i128)->Result<(),Error>{
        depositor.require_auth();
        if amount<=0{return Err(Error::ZeroAmount);}
        let prev:i128=env.storage().persistent().get(&DataKey::Deposit(depositor.clone())).unwrap_or(0);
        env.storage().persistent().set(&DataKey::Deposit(depositor.clone()),&(prev+amount));
        let pool:i128=env.storage().instance().get(&DataKey::PoolBalance).unwrap_or(0);
        env.storage().instance().set(&DataKey::PoolBalance,&(pool+amount));
        env.events().publish((symbol_short!("DEPOSIT"),depositor),amount);
        Ok(())
    }
    pub fn withdraw(env:Env,depositor:Address,amount:i128)->Result<(),Error>{
        depositor.require_auth();
        if amount<=0{return Err(Error::ZeroAmount);}
        let held:i128=env.storage().persistent().get(&DataKey::Deposit(depositor.clone())).unwrap_or(0);
        if held<amount{return Err(Error::InsufficientFunds);}
        let pool:i128=env.storage().instance().get(&DataKey::PoolBalance).unwrap_or(0);
        if pool<amount{return Err(Error::InsufficientFunds);}
        env.storage().persistent().set(&DataKey::Deposit(depositor.clone()),&(held-amount));
        env.storage().instance().set(&DataKey::PoolBalance,&(pool-amount));
        env.events().publish((symbol_short!("WITHDRAW"),depositor),amount);
        Ok(())
    }
    pub fn disburse(env:Env,admin:Address,recipient:Address,amount:i128,reason:String)->Result<(),Error>{
        admin.require_auth();
        let stored:Address=env.storage().instance().get(&DataKey::Admin).ok_or(Error::NotInitialized)?;
        if admin!=stored{return Err(Error::Unauthorized);}
        if amount<=0{return Err(Error::ZeroAmount);}
        let pool:i128=env.storage().instance().get(&DataKey::PoolBalance).unwrap_or(0);
        if pool<amount{return Err(Error::InsufficientFunds);}
        env.storage().instance().set(&DataKey::PoolBalance,&(pool-amount));
        env.events().publish((symbol_short!("DISBURSE"),recipient),(amount,reason));
        Ok(())
    }
    pub fn get_stats(env:Env)->FundStats{FundStats{pool_balance:env.storage().instance().get(&DataKey::PoolBalance).unwrap_or(0)}}
    pub fn get_deposit(env:Env,depositor:Address)->i128{env.storage().persistent().get(&DataKey::Deposit(depositor)).unwrap_or(0)}
}
#[cfg(test)]
mod test;
