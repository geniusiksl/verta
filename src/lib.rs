use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
    msg,
    // program_pack::{Pack, Sealed}, // Эти импорты не используются в текущем коде
    borsh::{BorshDeserialize, BorshSerialize},
    sysvar::{rent::Rent, Sysvar},
    program::{invoke_signed},
    system_instruction,
};

// Импортируем необходимые трейты напрямую из borsh, если solana_program не переэкспортирует их публично в этой версии
// (или оставляем импорт из solana_program::borsh, если он работает)
// В предыдущем шаге сработал вариант импорта через solana_program::borsh.
// Поэтому оставляем его. Если возникнут проблемы, попробуем импорт напрямую из borsh.
// use borsh::{BorshDeserialize, BorshSerialize}; // Эту строку удалили в предыдущем шаге

// Определение структуры аккаунта пользователя
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct UserAccount {
    pub karma: u64, // Количество кармы пользователя
    pub level: u8,  // Уровень пользователя
    // Можно добавить другие поля позже, например:
    // pub verified_contributions: u32, // Количество подтвержденных вкладов
    // pub registration_time: i64,     // Время регистрации
    // pub latest_contribution_type: u8, // Тип последнего вклада
}

// Определение размера структуры в байтах
// u64 = 8 байт, u8 = 1 байт. Общий размер: 8 + 1 = 9 байт.
impl UserAccount {
    pub const LEN: usize = 8 + 1; // Плюс потенциальные байты для других полей
}

// Определение возможных инструкций для нашей программы
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq)]
pub enum VertaInstruction {
    /// Инструкция 0: Регистрация пользователя.
    /// Создает или инициализирует аккаунт пользователя (PDA).
    RegisterUser,

    /// Инструкция 1: Добавить карму.
    /// Добавляет указанное количество кармы пользователю.
    /// Data: [1 (instruction_type), amount: u64]
    AddKarma { amount: u64 }, // Пример: добавить amount кармы

    /// Инструкция 2: Обновить уровень.
    /// Пересчитывает уровень пользователя на основе текущей кармы.
    /// Data: [2 (instruction_type)]
    UpdateLevel,

    // Можно добавить другие инструкции, например:
    // /// Инструкция 3: Подтвердить вклад другого пользователя
    // VerifyContribution { user_to_verify: Pubkey, contribution_id: u64 },
}

// Главная точка входа в программу
entrypoint!(process_instruction);

// Основная функция обработки инструкций
fn process_instruction(
    program_id: &Pubkey,        // ID вашей программы
    accounts: &[AccountInfo],   // Список аккаунтов, участвующих в транзакции
    instruction_data: &[u8],    // Данные инструкции (определяют, что делать)
) -> ProgramResult {
    msg!("Verta Program Entrypoint"); // Отладочное сообщение в начале

    // Десериализация данных инструкции
    let instruction = VertaInstruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("Received instruction: {:?}", instruction); // Отладочное сообщение с типом инструкции

    // Используем match для вызова нужной функции-обработчика
    match instruction {
        VertaInstruction::RegisterUser => {
            msg!("Processing RegisterUser instruction");
            process_register_user(program_id, accounts)
        }
        VertaInstruction::AddKarma { amount } => {
            msg!("Processing AddKarma instruction");
            process_add_karma(program_id, accounts, amount)
        }
        VertaInstruction::UpdateLevel => {
            msg!("Processing UpdateLevel instruction");
            process_update_level(program_id, accounts)
        }
        // Добавьте ветки для других инструкций
        // VertaInstruction::VerifyContribution { user_to_verify, contribution_id } => {
        //     msg!("Processing VerifyContribution instruction");
        //     process_verify_contribution(program_id, accounts, user_to_verify, contribution_id)
        // }
    }
}

// --- Функции-обработчики инструкций ---

// Обработчик инструкции RegisterUser
fn process_register_user(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Entering process_register_user");

    let accounts_iter = &mut accounts.iter();

    // Требуемые аккаунты для этой инструкции
    let user = next_account_info(accounts_iter)?; // Аккаунт пользователя (подписывает создание)
    let user_pda = next_account_info(accounts_iter)?; // PDA аккаунт для хранения данных
    let system_program = next_account_info(accounts_iter)?; // Системная программа для создания аккаунта

    // Проверки аккаунтов
    if !user.is_signer {
        msg!("User account must be a signer for registration");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Проверка PDA
    let (pda, bump) = Pubkey::find_program_address(&[b"user", user.key.as_ref()], program_id);
    if pda != *user_pda.key {
        msg!("Invalid PDA address provided for registration");
        return Err(ProgramError::InvalidArgument);
    }

    // Проверка, что аккаунт PDA не используется другой программой
    if user_pda.owner != program_id && !user_pda.data_is_empty() {
         msg!("Provided PDA account is owned by another program");
         return Err(ProgramError::IncorrectProgramId);
    }

    // Если аккаунт PDA пустой, создаем его
    if user_pda.data_is_empty() {
        msg!("Creating user account (PDA)");

        let space = UserAccount::LEN; // Размер аккаунта
        let rent_required = Rent::get()?.minimum_balance(space); // Требуемый баланс для ренты

        // Инструкция для создания аккаунта через системную программу
        let create_account_instruction = &system_instruction::create_account(
            user.key,          // Отправитель (пользователь)
            user_pda.key,      // Получатель (PDA)
            rent_required,     // Необходимый баланс для ренты
            space as u64,      // Размер аккаунта в байтах
            program_id,        // Владелец аккаунта (наша программа)
        );

        // Вызов инструкции создания аккаунта с подписью PDA
        invoke_signed(
            create_account_instruction,
            &[user.clone(), user_pda.clone(), system_program.clone()], // Аккаунты, участвующие в инструкции
            &[&[b"user", user.key.as_ref(), &[bump]]], // Сиды и бамп для подписи PDA
        )?;

        // Инициализация данных в новом аккаунте
        let account_data = UserAccount { karma: 0, level: 0 }; // Начальные значения кармы и уровня
        BorshSerialize::serialize(&account_data, &mut &mut user_pda.data.borrow_mut()[..])?;

        msg!("User account created and initialized successfully");

    } else {
        msg!("User account already exists. Skipping creation.");
        // Можно добавить логику для повторной инициализации, если нужно
    }

    Ok(()) // Успешное выполнение инструкции
}

// Обработчик инструкции AddKarma
fn process_add_karma(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    msg!("Entering process_add_karma");

    let accounts_iter = &mut accounts.iter();

    // Требуемые аккаунты: пользователь, которому добавляем карму, и его PDA
    let user_to_update_pda = next_account_info(accounts_iter)?; // PDA аккаунт пользователя
    // Возможно, потребуется аккаунт того, кто добавляет карму (верификатор)
    // let verifier = next_account_info(accounts_iter)?;

    // TODO: Добавить проверки аккаунтов (например, что user_to_update_pda принадлежит этой программе)
    // TODO: Реализовать логику проверки, кто может добавить карму (защита от абуза)
    // Например, проверить, что verifier подписал транзакцию и имеет достаточную репутацию.

    // Десериализуем данные аккаунта PDA
    let mut account_data = UserAccount::try_from_slice(&user_to_update_pda.data.borrow())?;
    
    // Обновляем карму
    account_data.karma += amount;
    msg!("Adding {} karma. New karma: {}", amount, account_data.karma);

    // Сериализуем обновленные данные обратно в аккаунт
    BorshSerialize::serialize(&account_data, &mut &mut user_to_update_pda.data.borrow_mut())?;

    // TODO: Возможно, здесь же вызывать process_update_level, или сделать это отдельной инструкцией

    msg!("AddKarma instruction processed successfully");

    Ok(()) // Успешное выполнение инструкции
}

// Обработчик инструкции UpdateLevel
fn process_update_level(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Entering process_update_level");

    let accounts_iter = &mut accounts.iter();

    // Требуемые аккаунты: пользователь, чей уровень обновляем, и его PDA
    let user_pda = next_account_info(accounts_iter)?; // PDA аккаунт пользователя

    // TODO: Добавить проверки аккаунтов (например, что user_pda принадлежит этой программе)

    // Десериализуем данные аккаунта PDA
    let mut account_data = UserAccount::try_from_slice(&user_pda.data.borrow())?;

    // TODO: Реализовать логику обновления уровня на основе account_data.karma
    // Пример очень простой логики:
    let new_level = (account_data.karma / 1000) as u8; // 1 уровень за каждые 1000 кармы
    if new_level > account_data.level {
        account_data.level = new_level;
        msg!("Level updated to {}", account_data.level);
        
        // Сериализуем обновленные данные обратно
         BorshSerialize::serialize(&account_data, &mut &mut user_pda.data.borrow_mut())?;
         msg!("User level updated successfully");
    } else {
        msg!("Level not changed. Current level: {}, required for next: {}", account_data.level, (account_data.level as u64 + 1) * 1000);
    }

    msg!("UpdateLevel instruction processed successfully");


    Ok(()) // Успешное выполнение инструкции
}

// TODO: Добавить другие функции-обработчики по мере необходимости (например, process_verify_contribution)ы