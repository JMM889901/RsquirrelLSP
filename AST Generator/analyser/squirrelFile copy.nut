//_loadouts_mp.gnut https://github.com/R2Northstar/NorthstarMods/blob/main/Northstar.CustomServers/mod/scripts/vscripts/_loadouts_mp.gnut
untyped
global function SvLoadoutsMP_Init

global function SetLoadoutGracePeriodEnabled
global function SetWeaponDropsEnabled

global function AddCallback_OnTryGetTitanLoadout
global function GetTitanLoadoutForPlayer

global struct sTryGetTitanLoadoutCallbackReturn
{
	bool wasChanged = false
	bool runMoreCallbacks = true
	TitanLoadoutDef& loadout
}

typedef TryGetTitanLoadoutCallbackType sTryGetTitanLoadoutCallbackReturn functionref( entity player, TitanLoadoutDef loadout, bool wasChanged )

struct {
	bool loadoutGracePeriodEnabled = true
	bool weaponDropsEnabled = true
	array< TryGetTitanLoadoutCallbackType > onTryGetTitanLoadoutCallbacks
	
	array<entity> dirtyLoadouts
} file

void function SvLoadoutsMP_Init()
{
	InitDefaultLoadouts() // titan loadout code relies on this, not called on server by default
		
	// most of these are fairly insecure right now, could break pdata if called maliciously, need fixing eventually
	RegisterSignal( "EndUpdateCachedLoadouts" )
	RegisterSignal( "GracePeriodDone" ) // temp to get weapons\_weapon_utility.nut:2271 to behave
	
	AddCallback_OnClientConnected( LoadoutsMPInitPlayer )
	
	AddClientCommandCallback( "RequestPilotLoadout", ClientCommandCallback_RequestPilotLoadout )
	AddClientCommandCallback( "RequestTitanLoadout", ClientCommandCallback_RequestTitanLoadout )
	AddClientCommandCallback( "SetPersistentLoadoutValue", ClientCommandCallback_SetPersistentLoadoutValue )
	AddClientCommandCallback( "SwapSecondaryAndWeapon3PersistentLoadoutData", ClientCommandCallback_SwapSecondaryAndWeapon3PersistentLoadoutData )
	AddClientCommandCallback( "SetBurnCardPersistenceSlot", ClientCommandCallback_SetBurnCardPersistenceSlot )
	
	if ( IsLobby() ) // can't usually set these in real games
	{
		AddClientCommandCallback( "SetCallsignIcon", ClientCommandCallback_SetCallsignIcon )
		AddClientCommandCallback( "SetCallsignCard", ClientCommandCallback_SetCallsignCard )
		AddClientCommandCallback( "SetFactionChoicePersistenceSlot", ClientCommandCallback_SetFactionChoicePersistenceSlot )
	}
	else
	{
		AddClientCommandCallback( "InGameMPMenuClosed", ClientCommandCallback_InGameMPMenuClosed )
		AddClientCommandCallback( "LoadoutMenuClosed", ClientCommandCallback_LoadoutMenuClosed )
	}
}

void function SetLoadoutGracePeriodEnabled( bool enabled )
{
	file.loadoutGracePeriodEnabled = enabled
}

void function SetWeaponDropsEnabled( bool enabled )
{
	if( enabled )
		FlagSet( "WeaponDropsAllowed" )
	else 
		FlagClear( "WeaponDropsAllowed" )
}

void function AddCallback_OnTryGetTitanLoadout( TryGetTitanLoadoutCallbackType callback )
{
	file.onTryGetTitanLoadoutCallbacks.append( callback )
}

TitanLoadoutDef function GetTitanLoadoutForPlayer( entity player )
{
	SetActiveTitanLoadout( player ) // set right loadout
	TitanLoadoutDef loadout = GetActiveTitanLoadout( player )

	// fix bug with titan weapons having null mods
	// null mods aren't valid and crash if we try to give them to npc
	loadout.primaryMods.removebyvalue( "null" )

	// allow scripts to modify loadouts
	bool wasChanged = false
	foreach ( TryGetTitanLoadoutCallbackType callback in file.onTryGetTitanLoadoutCallbacks )
	{
		sTryGetTitanLoadoutCallbackReturn callbackRet = callback( player, loadout, wasChanged )
		
		// whether the callback has changed the player's titan loadout
		wasChanged = wasChanged || callbackRet.wasChanged 
		if ( callbackRet.wasChanged )
			loadout = callbackRet.loadout
		
		// whether the callback has indicated that we should run no more callbacks ( e.g. if we're forcing a given loadout to be chosen, we shouldn't run any more )
		if ( !callbackRet.runMoreCallbacks )
			break
	}
	
	// do this again just in case
	loadout.primaryMods.removebyvalue( "null" )
	
	return loadout
}

void function LoadoutsMPInitPlayer( entity player )
{
	player.s.loadoutDirty <- false

	// these netints are required for callsigns and such to display correctly on other clients
	player.SetPlayerNetInt( "activeCallingCardIndex", player.GetPersistentVarAsInt( "activeCallingCardIndex" ) )
	player.SetPlayerNetInt( "activeCallsignIconIndex", player.GetPersistentVarAsInt( "activeCallsignIconIndex" ) )
}

// loadout clientcommands
bool function ClientCommandCallback_RequestPilotLoadout( entity player, array<string> args )
{
	if ( args.len() != 1 )
		return true
	
	print( player + " RequestPilotLoadout " + args[0] )
			
	// insecure, could be used to set invalid spawnloadout index potentially
	SetPersistentSpawnLoadoutIndex( player, "pilot", args[0].tointeger() )
	
	SetPlayerLoadoutDirty( player )
	
	return true
}

bool function ClientCommandCallback_RequestTitanLoadout( entity player, array<string> args )
{
	if ( args.len() != 1 )
		return true

	print( player + " RequestTitanLoadoutLoadout " + args[0] )
	
	// insecure, could be used to set invalid spawnloadout index potentially
	SetPersistentSpawnLoadoutIndex( player, "titan", args[0].tointeger() )
	
	if ( !IsLobby() )
		EarnMeterMP_SetTitanLoadout( player )
	
	return true
}

bool function ClientCommandCallback_SetPersistentLoadoutValue( entity player, array<string> args )
{
	//if ( args.len() != 4 )
	//	return true

	if ( args.len() < 4 )
		return true 
		
	string val = args[ 3 ]
	if ( args.len() > 4 ) // concat args after 3 into last arg so we can do strings with spaces and such
		for ( int i = 4; i < args.len(); i++ )
			val += " " + args[ i ]
	
	val = strip( val ) // remove any tailing whitespace

	print( player + " SetPersistentLoadoutValue " + args[0] + " " + args[1] + " " + args[2] + " " + val )
	
	// VERY temp and insecure
	SetPersistentLoadoutValue( player, args[0], args[1].tointeger(), args[2], val )
	
	print( args[ 0 ] )
	if ( args[0] == "pilot" )
		SetPlayerLoadoutDirty( player ) 
	
	UnlockAchievement( player, achievements.CUSTOMIZE_LOADOUT )
	
	return true
}

bool function ClientCommandCallback_SwapSecondaryAndWeapon3PersistentLoadoutData( entity player, array<string> args )
{
	if ( args.len() != 1 )
		return true
		
	print( "SwapSecondaryAndWeapon3PersistentLoadoutData " + args[0] )
	
	// get loadout
	int index = args[0].tointeger()
	
	if ( !IsValidPilotLoadoutIndex(index) )
		return false

	PilotLoadoutDef loadout = GetPilotLoadoutFromPersistentData( player, index )

	// swap loadouts
	// is this a good way of doing it? idk i think this is the best way of doing it
	// can't use validation because when you swap, you'll have a secondary/weapon3 in 2 slots at once at one point, which fails validation
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "secondary", loadout.weapon3 )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "secondaryMod1", loadout.weapon3Mod1 )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "secondaryMod2", loadout.weapon3Mod2 )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "secondaryMod3", loadout.weapon3Mod3 )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "secondarySkinIndex", loadout.weapon3SkinIndex.tostring() )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "secondaryCamoIndex", loadout.weapon3CamoIndex.tostring() )
	
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "weapon3", loadout.secondary )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "weapon3Mod1", loadout.secondaryMod1 )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "weapon3Mod2", loadout.secondaryMod2 )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "weapon3Mod3", loadout.secondaryMod3 )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "weapon3SkinIndex", loadout.secondarySkinIndex.tostring() )
	SetPlayerPersistentVarWithoutValidation( player, "pilot", index, "weapon3CamoIndex", loadout.secondaryCamoIndex.tostring() )
		
	SetPlayerLoadoutDirty( player )
	
	return true
}

bool function ClientCommandCallback_SetBurnCardPersistenceSlot( entity player, array<string> args )
{
	if ( args.len() != 1 || GetGameState() >= eGameState.Playing )
		return true
	
	print( player + " SetBurnCardPersistenceSlot " + args[0] )
	
	if ( IsRefValidAndOfType( args[0], eItemTypes.BURN_METER_REWARD ) )
		player.SetPersistentVar( "burnmeterSlot", BurnReward_GetByRef( args[0] ).id )
	else
		print( player + " invalid ref " + args[0] )
	
	return true
}

// lobby clientcommands
bool function ClientCommandCallback_SetCallsignIcon( entity player, array<string> args )
{
	print( player + " SetCallsignIcon " + args[0] )

	if ( IsRefValidAndOfType( args[0], eItemTypes.CALLSIGN_ICON ) )
		PlayerCallsignIcon_SetActiveByRef( player, args[0] )
	else
		print( player + " invalid ref " + args[0] )
	
	return true
}

bool function ClientCommandCallback_SetCallsignCard( entity player, array<string> args )
{
	print( player + " SetCallsignIcon " + args[0] )

	if ( IsRefValidAndOfType( args[0], eItemTypes.CALLING_CARD ) )
		PlayerCallingCard_SetActiveByRef( player, args[0] )
	else
		print( player + " invalid ref " + args[0] )
	
	return true
}

bool function ClientCommandCallback_SetFactionChoicePersistenceSlot( entity player, array<string> args )
{
	print( player + " SetFactionChoicePersistenceSlot " + args[0] )

	if ( IsRefValidAndOfType( args[0], eItemTypes.FACTION ) )
		player.SetPersistentVar( "factionChoice", args[0] ) // no function for this so gotta set directly lol
	
	return true
}

bool function ClientCommandCallback_LoadoutMenuClosed( entity player, array<string> args )
{
	TryGivePilotLoadoutForGracePeriod( player )
	return true
}

bool function ClientCommandCallback_InGameMPMenuClosed( entity player, array<string> args )
{
	TryGivePilotLoadoutForGracePeriod( player )
	return true
}

bool function IsRefValidAndOfType( string ref, int itemType )
{
	return IsRefValid( ref ) && GetItemType( ref ) == itemType 
}

void function SetPlayerLoadoutDirty( entity player )
{
	if ( !IsLobby() )
		player.s.loadoutDirty = true
}

void function TryGivePilotLoadoutForGracePeriod( entity player )
{
	if ( !IsLobby() && IsAlive( player ) && player.s.loadoutDirty && !player.IsTitan() && !player.ContextAction_IsActive() )
	{
		player.s.loadoutDirty = false
	
		// for intros
		float respawnTimeReal
		if ( GetGameState() == eGameState.Playing && Time() - expect float( GetServerVar( "gameStateChangeTime" ) ) <= CLASS_CHANGE_GRACE_PERIOD )
			respawnTimeReal = expect float( GetServerVar( "gameStateChangeTime" ) )
		else
			respawnTimeReal = expect float( player.s.respawnTime )
		
		if ( ( ( Time() - respawnTimeReal <= CLASS_CHANGE_GRACE_PERIOD || GetGameState() < eGameState.Playing ) && file.loadoutGracePeriodEnabled ) || player.p.usingLoadoutCrate )
		{
			if ( !Loadouts_CanGivePilotLoadout( player ) && player.GetParent() != null && ( HasCinematicFlag( player, CE_FLAG_INTRO ) || HasCinematicFlag( player, CE_FLAG_CLASSIC_MP_SPAWNING ) || HasCinematicFlag( player, CE_FLAG_WAVE_SPAWNING ) ) )
				thread GiveLoadoutWhenIntroOver( player )
			else
				Loadouts_TryGivePilotLoadout( player )
			
			player.p.usingLoadoutCrate = false
		}
		else
			SendHudMessage( player, "#LOADOUT_CHANGE_NEXT_BOTH", -1, 0.4, 255, 255, 255, 255, 0.15, 3.0, 0.5 ) // like 90% sure this is innacurate lol
	}
}

void function GiveLoadoutWhenIntroOver( entity player )
{
	player.EndSignal( "OnDestroy" )
	player.EndSignal( "OnDeath" )
	
	while ( player.GetParent() != null && ( HasCinematicFlag( player, CE_FLAG_INTRO ) || HasCinematicFlag( player, CE_FLAG_CLASSIC_MP_SPAWNING ) || HasCinematicFlag( player, CE_FLAG_WAVE_SPAWNING ) ) )
		WaitFrame()
	
	Loadouts_TryGivePilotLoadout( player )
}	